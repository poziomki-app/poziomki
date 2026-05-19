//! Admin-only endpoints gated on a shared `ADMIN_TOKEN` env var.
//!
//! During invite-only beta the admin surface is intentionally tiny —
//! just account suspension (soft ban) for moderation response. See
//! `.github/MODERATION.md` for the policy this implements.
//!
//! Auth model: the `X-Admin-Token` header must match the
//! `ADMIN_TOKEN` environment variable, compared in constant time.
//! A bearer session is **not** required — operators may be acting
//! without a normal user account. This is a deliberately simple
//! check; the expected caller is a single trusted operator invoking
//! the endpoint from a secure host.
//!
//! The env var is missing → 503. That keeps dev / CI from exposing
//! the endpoint by accident.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::app::AppContext;
use chrono::Utc;
use diesel::sql_types::{Integer, Nullable, Text, Uuid as SqlUuid};
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::api::{error_response, state, ErrorSpec};

#[derive(diesel::deserialize::QueryableByName)]
struct BanResult {
    #[diesel(sql_type = Nullable<Integer>)]
    user_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BanBody {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BroadcastBody {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub deep_link: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FeatureEventBody {
    pub is_featured: bool,
}

fn admin_auth(headers: &HeaderMap) -> Result<(), Box<Response>> {
    let Some(expected) = std::env::var("ADMIN_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            headers,
            ErrorSpec {
                error: "Admin endpoint disabled".to_string(),
                code: "ADMIN_DISABLED",
                details: None,
            },
        )));
    };
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Constant-time compare so an attacker probing the header can't
    // derive bytes of the expected token from response timing.
    if expected.as_bytes().ct_eq(provided.as_bytes()).unwrap_u8() != 1 {
        return Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            headers,
            ErrorSpec {
                error: "Admin token rejected".to_string(),
                code: "ADMIN_FORBIDDEN",
                details: None,
            },
        )));
    }
    Ok(())
}

pub(super) async fn ban_user(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(pid): Path<Uuid>,
    Json(body): Json<BanBody>,
) -> Response {
    // Rate limit by caller IP before token compare — caps online
    // brute-force of ADMIN_TOKEN from any single source even though
    // the compare itself is constant time.
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::AdminAuth,
    )
    .await
    {
        return *resp;
    }
    if let Err(resp) = admin_auth(&headers) {
        return *resp;
    }

    let Ok(mut conn) = crate::db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "DB unavailable".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        );
    };

    let now = Utc::now();
    let reason = body.reason.unwrap_or_else(|| "no reason provided".into());

    // The admin request has no viewer context, so a plain UPDATE /
    // DELETE through RLS would filter to zero rows (users policy +
    // sessions policy both require app.current_user_id()). Route
    // through app.admin_ban_user, a SECURITY DEFINER helper that
    // does the pid resolution + UPDATE + session purge in one
    // owner-privileged transaction. Returns the internal user_id
    // (for cache invalidation) or NULL if the pid doesn't exist.
    let ban = diesel::sql_query("SELECT app.admin_ban_user($1, $2) AS user_id")
        .bind::<SqlUuid, _>(pid)
        .bind::<Text, _>(&reason)
        .get_result::<BanResult>(&mut conn)
        .await;

    let user_id = match ban {
        Ok(row) => match row.user_id {
            Some(id) => id,
            None => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &headers,
                    ErrorSpec {
                        error: "User not found".to_string(),
                        code: "NOT_FOUND",
                        details: None,
                    },
                );
            }
        },
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "Ban write failed".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            );
        }
    };

    state::invalidate_auth_cache_for_user_id(user_id).await;
    // Kick any live WebSocket connection belonging to the banned
    // user — the DB + cache path already stops future requests,
    // this closes the last remaining transport on existing sockets.
    ctx.chat_hub.disconnect_user(user_id);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "banned_at": now })),
    )
        .into_response()
}

/// Fan-out broadcast push to every registered device. Bypasses
/// per-user notification preferences — operators use this for
/// announcements, outages, etc., not for normal product traffic.
pub(super) async fn broadcast_push(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<BroadcastBody>,
) -> Response {
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::AdminAuth,
    )
    .await
    {
        return *resp;
    }
    if let Err(resp) = admin_auth(&headers) {
        return *resp;
    }

    let title = body.title.trim();
    let body_text = body.body.trim();
    if title.is_empty() || body_text.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "title and body are required".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        );
    }

    let (delivered, rejected) = crate::push::fcm::send_broadcast(
        title.to_string(),
        body_text.to_string(),
        body.deep_link.clone().filter(|s| !s.trim().is_empty()),
    )
    .await;

    tracing::info!(
        delivered,
        rejected,
        deep_link = ?body.deep_link.as_deref(),
        "fcm_broadcast_sent"
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "delivered": delivered,
            "rejected": rejected,
        })),
    )
        .into_response()
}

/// Toggle an event's `is_featured` flag. The public event-create payload
/// does NOT accept this — only operators can mark an event featured via
/// this endpoint. Featured events sort to the top of the wydarzenia list
/// and render with a "wyróżnione" badge.
pub(super) async fn feature_event(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(body): Json<FeatureEventBody>,
) -> Response {
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::AdminAuth,
    )
    .await
    {
        return *resp;
    }
    if let Err(resp) = admin_auth(&headers) {
        return *resp;
    }

    let Ok(mut conn) = crate::db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "DB unavailable".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        );
    };

    // events is RLS-gated by the viewer policy; admin has no viewer
    // context. Route through a narrow SECURITY DEFINER helper.
    let rows = diesel::sql_query("SELECT app.admin_set_event_featured($1, $2) AS updated")
        .bind::<SqlUuid, _>(event_id)
        .bind::<diesel::sql_types::Bool, _>(body.is_featured)
        .execute(&mut conn)
        .await;

    match rows {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "event_id": event_id,
                "is_featured": body.is_featured,
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "admin feature_event failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "feature toggle failed".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        }
    }
}
