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
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::api::{error_response, state, ErrorSpec};
use crate::db::schema::{sessions, users};

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

    // Resolve pid → id so the admin caller never touches internal ids
    // and the audit log records a pid that's safe to share externally.
    let Ok(user_id) = users::table
        .filter(users::pid.eq(pid))
        .select(users::id)
        .first::<i32>(&mut conn)
        .await
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "User not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        );
    };

    let now = Utc::now();
    let reason = body.reason.unwrap_or_else(|| "no reason provided".into());

    // Flip the ban flag + invalidate every active session in one tx
    // so a banned user can't keep using a cached session past the
    // ban. The auth middleware also rejects banned users on the
    // next cache miss, but session purge makes the kick immediate.
    let result: Result<(), diesel::result::Error> = async {
        diesel::update(users::table.filter(users::id.eq(user_id)))
            .set((
                users::banned_at.eq(Some(now)),
                users::banned_reason.eq(Some(reason.clone())),
                users::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .await?;
        diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
    .await;

    if result.is_err() {
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

    state::invalidate_auth_cache_for_user_id(user_id).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({ "banned_at": now })),
    )
        .into_response()
}
