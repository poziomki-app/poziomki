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
use serde::Deserialize;
use uuid::Uuid;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::app::AppContext;

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
/// before any extractors, so it can reject hostile origins *before* Axum's
/// `WebSocketUpgrade` extractor runs its own preconditions — which matters
/// both for security (reject early, no socket work) and for testability
/// (in-process test harnesses can't satisfy the WebSocket extractor's HTTP
/// version check, so any gate that lives *inside* the handler body is
/// unreachable from integration tests).
pub async fn ws_upgrade_gate(req: Request, next: Next) -> Response {
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        if !is_allowed_ws_origin(origin) {
            tracing::warn!(origin = %origin, "ws_upgrade: rejected origin");
            return (StatusCode::FORBIDDEN, "forbidden origin").into_response();
        }
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

    // Resolve target user's internal id
    let mut conn = crate::db::conn().await?;
    let target_user_id: Option<i32> = users::table
        .filter(users::pid.eq(target_pid))
        .select(users::id)
        .first(&mut conn)
        .await
        .optional()?;

    let Some(target_user_id) = target_user_id else {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "User not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    if user.id == target_user_id {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Cannot create DM with yourself".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let conversation = conversations::resolve_or_create_dm(user.id, target_user_id).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": {
                "conversationId": conversation.id.to_string(),
            }
        })),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// REST: Resolve event conversation
// ---------------------------------------------------------------------------

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

    let mut conn = crate::db::conn().await?;

    // Load event
    let event: Option<(Uuid, String, Uuid)> = events::table
        .filter(events::id.eq(event_id))
        .select((events::id, events::title, events::creator_id))
        .first(&mut conn)
        .await
        .optional()?;

    let Some((event_id, event_title, creator_profile_id)) = event else {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Event not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    // Check if user is creator or attendee
    let user_profile_id: Option<Uuid> = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .select(profiles::id)
        .first(&mut conn)
        .await
        .optional()?;

    let Some(user_profile_id) = user_profile_id else {
        return Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: "Profile required".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        ));
    };

    let is_creator = creator_profile_id == user_profile_id;
    let is_attendee: bool = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::profile_id.eq(user_profile_id))
        .filter(event_attendees::status.eq("going"))
        .count()
        .get_result::<i64>(&mut conn)
        .await?
        > 0;

    if !is_creator && !is_attendee {
        return Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: "Must be creator or attendee".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        ));
    }

    // Resolve creator's user_id from their profile_id
    let creator_user_id: i32 = profiles::table
        .filter(profiles::id.eq(creator_profile_id))
        .select(profiles::user_id)
        .first(&mut conn)
        .await?;

    let conversation = conversations::resolve_or_create_event_conversation(
        event_id,
        &event_title,
        creator_user_id,
    )
    .await?;

    // Ensure requesting user is a member
    diesel::insert_into(conversation_members::table)
        .values(&NewConversationMember {
            conversation_id: conversation.id,
            user_id: user.id,
            joined_at: chrono::Utc::now(),
        })
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": {
                "conversationId": conversation.id.to_string(),
            }
        })),
    )
        .into_response())
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

    let mut conn = crate::db::conn().await?;
    let now = chrono::Utc::now();

    diesel::insert_into(push_subscriptions::table)
        .values(
            &crate::db::models::push_subscriptions::NewPushSubscription {
                id: uuid::Uuid::new_v4(),
                user_id: user.id,
                device_id: body.device_id.clone(),
                ntfy_topic: body.ntfy_topic.clone(),
                created_at: now,
            },
        )
        .on_conflict((push_subscriptions::user_id, push_subscriptions::device_id))
        .do_update()
        .set(push_subscriptions::ntfy_topic.eq(&body.ntfy_topic))
        .execute(&mut conn)
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

    let mut conn = crate::db::conn().await?;
    diesel::delete(
        push_subscriptions::table
            .filter(push_subscriptions::user_id.eq(user.id))
            .filter(push_subscriptions::device_id.eq(&body.device_id)),
    )
    .execute(&mut conn)
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
