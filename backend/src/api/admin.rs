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
