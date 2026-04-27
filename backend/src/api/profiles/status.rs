//! Lightweight ephemeral-status endpoint.
//!
//! POST /api/v1/profiles/me/status with `{ emoji?, text? }` sets the
//! caller's `status_text` + `status_emoji` and stamps a 24h expiry. Empty
//! body (or both fields null/blank) clears it.
//!
//! This is deliberately a separate endpoint from PATCH /profiles/{id}
//! so the Poznaj composer can write a status without re-validating
//! every field on the profile and without burning moderation CPU on
//! the bio. The text is still moderated — same Bielik-Guard gate as
//! profile bio/status — but the response shape is minimal (just the
//! refreshed status fields the client needs to render the pill).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::api::auth_or_respond;
use crate::api::common::{error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;
use crate::db::schema::profiles;

type Result<T> = crate::error::AppResult<T>;

#[derive(Debug, Deserialize, Clone)]
pub(in crate::api) struct SetStatusBody {
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub(in crate::api) struct StatusResponse {
    pub status: Option<String>,
    pub status_emoji: Option<String>,
    pub status_expires_at: Option<DateTime<Utc>>,
}

const STATUS_TTL_HOURS: i64 = 24;
const STATUS_TEXT_MAX_CHARS: usize = 160;
const STATUS_EMOJI_MAX_BYTES: usize = 32;

pub(in crate::api) async fn set_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SetStatusBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let trimmed_text = payload
        .text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let trimmed_emoji = payload
        .emoji
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(ref t) = trimmed_text {
        if t.chars().count() > STATUS_TEXT_MAX_CHARS {
            return Ok(error_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                &headers,
                ErrorSpec {
                    error: format!("Status must be at most {STATUS_TEXT_MAX_CHARS} characters"),
                    code: "VALIDATION_ERROR",
                    details: None,
                },
            ));
        }
    }
    if let Some(ref e) = trimmed_emoji {
        if e.len() > STATUS_EMOJI_MAX_BYTES {
            return Ok(error_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                &headers,
                ErrorSpec {
                    error: "Emoji is too long".to_string(),
                    code: "VALIDATION_ERROR",
                    details: None,
                },
            ));
        }
    }

    let user_id = user.id;
    let user_id_for_response = user.id;

    // Both null/blank → clear. We could 404 the request, but the
    // mobile composer's "clear" button benefits from a uniform code
    // path that always returns the canonical "no status" payload.
    let (text_to_persist, emoji_to_persist, expires_to_persist) =
        if trimmed_text.is_none() && trimmed_emoji.is_none() {
            (None, None, None)
        } else {
            let expiry = Utc::now() + Duration::hours(STATUS_TTL_HOURS);
            (trimmed_text, trimmed_emoji, Some(expiry))
        };

    let updated = db::with_viewer_tx(viewer, move |conn| {
        async move {
            diesel::update(profiles::table.filter(profiles::user_id.eq(user_id)))
                .set((
                    profiles::status_text.eq(text_to_persist.as_ref()),
                    profiles::status_emoji.eq(emoji_to_persist.as_ref()),
                    profiles::status_expires_at.eq(expires_to_persist),
                    profiles::updated_at.eq(Utc::now()),
                ))
                .returning((
                    profiles::status_text,
                    profiles::status_emoji,
                    profiles::status_expires_at,
                ))
                .get_result::<(Option<String>, Option<String>, Option<DateTime<Utc>>)>(conn)
                .await
                .optional()
        }
        .scope_boxed()
    })
    .await?;

    let _ = user_id_for_response;
    let Some((status, status_emoji, status_expires_at)) = updated else {
        return Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Profile not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    Ok(Json(StatusResponse {
        status,
        status_emoji,
        status_expires_at,
    })
    .into_response())
}
