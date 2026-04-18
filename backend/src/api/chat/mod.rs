pub mod conversations;
pub mod hub;
pub mod messages;
pub mod protocol;
pub mod push;
pub mod report_handler;
pub mod report_repo;
pub mod ws;

use axum::{
    extract::{ws::WebSocketUpgrade, Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;

type Result<T> = crate::error::AppResult<T>;

// ---------------------------------------------------------------------------
// WebSocket upgrade
// ---------------------------------------------------------------------------

/// Origins the browser-side app is allowed to open a chat socket from.
/// Native mobile clients typically omit `Origin`; that is accepted. Any
/// *present* Origin that doesn't match the allowlist is rejected with 403
/// before we even start the upgrade, so a malicious page can't hold the
/// socket open during the 5s auth window.
const ALLOWED_WS_ORIGINS: &[&str] = &[
    "https://poziomki.app",
    "https://www.poziomki.app",
    "https://mobile.poziomki.app",
    "https://api.poziomki.app",
    "http://localhost",
    "http://127.0.0.1",
];

fn is_allowed_ws_origin(origin: &str) -> bool {
    ALLOWED_WS_ORIGINS.iter().any(|allowed| {
        // Exact match (`https://poziomki.app`) *or* a localhost-with-port
        // prefix (`http://localhost:5173`). We deliberately don't match
        // other schemes or subdomain suffixes.
        origin == *allowed
            || ((allowed.starts_with("http://localhost")
                || allowed.starts_with("http://127.0.0.1"))
                && origin.starts_with(&format!("{allowed}:")))
    })
}

pub async fn ws_upgrade(State(ctx): State<AppContext>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| ws::handle_socket(socket, ctx.chat_hub))
}

/// Route-level gate for the `/chat/ws` endpoint. Runs on the raw `Request`
/// before any extractors, so both the origin allowlist and the IP rate
/// limit fire uniformly in prod and in axum-test (the in-process transport
/// can't satisfy `WebSocketUpgrade`'s HTTP-version precondition, so any
/// check that lives inside the handler body is unreachable from integration
/// tests). Origin is checked first — cheap and local, keeps hostile pages
/// from burning a rate-limit slot.
pub async fn ws_upgrade_gate(req: Request, next: Next) -> Response {
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        if !is_allowed_ws_origin(origin) {
            tracing::warn!(origin = %origin, "ws_upgrade: rejected origin");
            return (StatusCode::FORBIDDEN, "forbidden origin").into_response();
        }
    }
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        req.headers(),
        crate::api::ip_rate_limit::IpRateLimitAction::ChatWsUpgrade,
    )
    .await
    {
        return *response;
    }
    next.run(req).await
}

#[cfg(test)]
mod origin_tests {
    use super::is_allowed_ws_origin;

    #[test]
    fn accepts_allowlisted() {
        assert!(is_allowed_ws_origin("https://poziomki.app"));
        assert!(is_allowed_ws_origin("https://mobile.poziomki.app"));
        assert!(is_allowed_ws_origin("https://api.poziomki.app"));
        assert!(is_allowed_ws_origin("http://localhost:5173"));
        assert!(is_allowed_ws_origin("http://127.0.0.1:3000"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!is_allowed_ws_origin("https://evil.com"));
        assert!(!is_allowed_ws_origin("https://poziomki.app.evil.com"));
        assert!(!is_allowed_ws_origin("http://poziomki.app"));
        assert!(!is_allowed_ws_origin("capacitor://localhost"));
    }
}

// ---------------------------------------------------------------------------
// REST: Resolve/create DM conversation
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct DmRequest {
    #[serde(rename = "userId")]
    user_id: String,
}

pub async fn resolve_dm(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<DmRequest>,
) -> Result<Response> {
    use crate::db::schema::users;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let (_session, user) = auth_or_respond!(headers);

    let target_pid = match parse_uuid_response(&body.user_id, "user", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let caller_user_id = user.id;

    let outcome: std::result::Result<Option<Uuid>, &'static str> =
        db::with_viewer_tx(viewer, move |conn| {
            async move {
                let target_user_id: Option<i32> = users::table
                    .filter(users::pid.eq(target_pid))
                    .select(users::id)
                    .first(conn)
                    .await
                    .optional()
                    .map_err(|_| diesel::result::Error::RollbackTransaction)?;

                let Some(target_user_id) = target_user_id else {
                    return Ok::<
                        std::result::Result<Option<Uuid>, &'static str>,
                        diesel::result::Error,
                    >(Err("target_missing"));
                };

                if caller_user_id == target_user_id {
                    return Ok(Err("self_dm"));
                }

                let conversation =
                    conversations::resolve_or_create_dm(conn, caller_user_id, target_user_id)
                        .await
                        .map_err(|_| diesel::result::Error::RollbackTransaction)?;

                Ok(Ok(Some(conversation.id)))
            }
            .scope_boxed()
        })
        .await?;

    match outcome {
        Err("target_missing") => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "User not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        Err("self_dm") => Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Cannot create DM with yourself".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        )),
        Err(_) => Ok(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "Internal error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        )),
        Ok(Some(conv_id)) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "data": {
                    "conversationId": conv_id.to_string(),
                }
            })),
        )
            .into_response()),
        Ok(None) => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// REST: Resolve event conversation
// ---------------------------------------------------------------------------

enum EventConvOutcome {
    NotFound,
    NoProfile,
    Forbidden,
    Ok(Uuid),
}

pub async fn resolve_event_conversation(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Result<Response> {
    use crate::db::models::conversation_members::NewConversationMember;
    use crate::db::schema::{conversation_members, event_attendees, events, profiles};
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let (_session, user) = auth_or_respond!(headers);

    let event_id = match parse_uuid_response(&event_id, "event", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let caller_user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let event: Option<(Uuid, String, Uuid)> = events::table
                .filter(events::id.eq(event_id))
                .select((events::id, events::title, events::creator_id))
                .first(conn)
                .await
                .optional()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let Some((event_id, event_title, creator_profile_id)) = event else {
                return Ok::<_, diesel::result::Error>(EventConvOutcome::NotFound);
            };

            let user_profile_id: Option<Uuid> = profiles::table
                .filter(profiles::user_id.eq(caller_user_id))
                .select(profiles::id)
                .first(conn)
                .await
                .optional()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let Some(user_profile_id) = user_profile_id else {
                return Ok(EventConvOutcome::NoProfile);
            };

            let is_creator = creator_profile_id == user_profile_id;
            let is_attendee: bool = event_attendees::table
                .filter(event_attendees::event_id.eq(event_id))
                .filter(event_attendees::profile_id.eq(user_profile_id))
                .filter(event_attendees::status.eq("going"))
                .count()
                .get_result::<i64>(conn)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
                > 0;

            if !is_creator && !is_attendee {
                return Ok(EventConvOutcome::Forbidden);
            }

            let creator_user_id: i32 = profiles::table
                .filter(profiles::id.eq(creator_profile_id))
                .select(profiles::user_id)
                .first(conn)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let conversation = conversations::resolve_or_create_event_conversation(
                conn,
                event_id,
                &event_title,
                creator_user_id,
            )
            .await
            .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            diesel::insert_into(conversation_members::table)
                .values(&NewConversationMember {
                    conversation_id: conversation.id,
                    user_id: caller_user_id,
                    joined_at: chrono::Utc::now(),
                })
                .on_conflict_do_nothing()
                .execute(conn)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            Ok(EventConvOutcome::Ok(conversation.id))
        }
        .scope_boxed()
    })
    .await?;

    match outcome {
        EventConvOutcome::NotFound => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Event not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        EventConvOutcome::NoProfile => Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: "Profile required".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        )),
        EventConvOutcome::Forbidden => Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: "Must be creator or attendee".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        )),
        EventConvOutcome::Ok(conv_id) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "data": {
                    "conversationId": conv_id.to_string(),
                }
            })),
        )
            .into_response()),
    }
}

// ---------------------------------------------------------------------------
// REST: Push subscription management
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PushRegisterRequest {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "ntfyTopic")]
    ntfy_topic: String,
}

pub async fn push_register(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<PushRegisterRequest>,
) -> Result<Response> {
    use crate::db::schema::push_subscriptions;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let (_session, user) = auth_or_respond!(headers);

    if body.device_id.is_empty() || body.device_id.len() > 64 {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "invalid device_id".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    if body.ntfy_topic.is_empty()
        || body.ntfy_topic.len() > 128
        || !body
            .ntfy_topic
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "invalid ntfy_topic".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let device_id = body.device_id.clone();
    let ntfy_topic = body.ntfy_topic.clone();

    db::with_viewer_tx(viewer, move |conn| {
        async move {
            let now = chrono::Utc::now();
            diesel::insert_into(push_subscriptions::table)
                .values(
                    &crate::db::models::push_subscriptions::NewPushSubscription {
                        id: uuid::Uuid::new_v4(),
                        user_id,
                        device_id: device_id.clone(),
                        ntfy_topic: ntfy_topic.clone(),
                        created_at: now,
                    },
                )
                .on_conflict((push_subscriptions::user_id, push_subscriptions::device_id))
                .do_update()
                .set(push_subscriptions::ntfy_topic.eq(&ntfy_topic))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response())
}

#[derive(Deserialize)]
pub struct PushUnregisterRequest {
    #[serde(rename = "deviceId")]
    device_id: String,
}

pub async fn push_unregister(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<PushUnregisterRequest>,
) -> Result<Response> {
    use crate::db::schema::push_subscriptions;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let (_session, user) = auth_or_respond!(headers);

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let device_id = body.device_id.clone();

    db::with_viewer_tx(viewer, move |conn| {
        async move {
            diesel::delete(
                push_subscriptions::table
                    .filter(push_subscriptions::user_id.eq(user_id))
                    .filter(push_subscriptions::device_id.eq(&device_id)),
            )
            .execute(conn)
            .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response())
}

// ---------------------------------------------------------------------------
// REST: Chat config
// ---------------------------------------------------------------------------

pub async fn chat_config(State(_ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let _ = auth_or_respond!(headers);

    let ntfy_server = crate::api::chat::push::resolved_ntfy_server();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": {
                "chatMode": "ws",
                "ntfyServer": ntfy_server,
            }
        })),
    )
        .into_response())
}
