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
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct StatusResponse {
    pub status: Option<String>,
    pub status_emoji: Option<String>,
    pub status_expires_at: Option<DateTime<Utc>>,
}

const STATUS_TTL_HOURS: i64 = 24;
const STATUS_TEXT_MAX_CHARS: usize = 160;
const STATUS_EMOJI_MAX_BYTES: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum NormalizedStatus {
    /// Both fields null/blank — caller wants to clear the status.
    Clear,
    /// At least one field present and within limits.
    Set {
        text: Option<String>,
        emoji: Option<String>,
    },
    /// Caller violated a length cap.
    InvalidTextTooLong,
    InvalidEmojiTooLong,
}

/// Trim whitespace, fold blanks to `None`, and validate length caps.
///
/// Pulled out of the handler so the validation rules can be unit-tested
/// without spinning up DB / moderation / auth — those each have their
/// own coverage and the bug class we care about here is "did a blank
/// payload route to clear?" / "is the cap enforced on graphemes vs
/// bytes correctly?".
pub(super) fn normalize_status(payload: &SetStatusBody) -> NormalizedStatus {
    let text = payload
        .text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let emoji = payload
        .emoji
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(ref t) = text {
        if t.chars().count() > STATUS_TEXT_MAX_CHARS {
            return NormalizedStatus::InvalidTextTooLong;
        }
    }
    if let Some(ref e) = emoji {
        if e.len() > STATUS_EMOJI_MAX_BYTES {
            return NormalizedStatus::InvalidEmojiTooLong;
        }
    }

    if text.is_none() && emoji.is_none() {
        NormalizedStatus::Clear
    } else {
        NormalizedStatus::Set { text, emoji }
    }
}

pub(in crate::api) async fn set_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SetStatusBody>,
) -> Result<Response> {
    // Bielik moderation runs on every set, so rate-limit before auth lookup.
    // Keyed on client IP — caller is authed but the IP-bucket prevents one
    // logged-in client from fanning out across many sessions and still
    // amortising the inference CPU cost.
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::ProfileStatus,
    )
    .await
    {
        return Ok(*resp);
    }

    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let (trimmed_text, trimmed_emoji, is_clear) = match normalize_status(&payload) {
        NormalizedStatus::Clear => (None, None, true),
        NormalizedStatus::Set { text, emoji } => (text, emoji, false),
        NormalizedStatus::InvalidTextTooLong => {
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
        NormalizedStatus::InvalidEmojiTooLong => {
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
    };

    // Moderate the free-text portion through the same Bielik-Guard gate
    // as bio. A 24h ephemeral status reaches every viewer the user shows
    // up to, so the same content rules apply. Emoji is not moderated —
    // we control the picker palette client-side.
    if let Some(ref t) = trimmed_text {
        match super::profiles_write_handler::moderate_profile_text(
            t,
            super::profiles_write_handler::ProfileTextField::Status,
            &headers,
        )
        .await
        {
            Ok(None) => {}
            Ok(Some(rejection)) => return Ok(rejection),
            Err(error) => {
                tracing::error!(%error, "status moderation failed; rejecting save");
                return Err(error);
            }
        }
    }

    let user_id = user.id;

    // Both null/blank → clear. We could 404 the request, but the
    // mobile composer's "clear" button benefits from a uniform code
    // path that always returns the canonical "no status" payload.
    let (text_to_persist, emoji_to_persist, expires_to_persist) = if is_clear {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: Option<&str>, emoji: Option<&str>) -> SetStatusBody {
        SetStatusBody {
            text: text.map(str::to_string),
            emoji: emoji.map(str::to_string),
        }
    }

    #[test]
    fn both_blank_yields_clear() {
        assert_eq!(normalize_status(&body(None, None)), NormalizedStatus::Clear);
        assert_eq!(
            normalize_status(&body(Some("   "), Some("\t"))),
            NormalizedStatus::Clear
        );
        assert_eq!(
            normalize_status(&body(Some(""), Some(""))),
            NormalizedStatus::Clear
        );
    }

    #[test]
    fn trimmed_text_kept() {
        assert_eq!(
            normalize_status(&body(Some("  hi  "), None)),
            NormalizedStatus::Set {
                text: Some("hi".to_string()),
                emoji: None,
            }
        );
    }

    #[test]
    fn emoji_only_is_set() {
        assert_eq!(
            normalize_status(&body(None, Some("🎉"))),
            NormalizedStatus::Set {
                text: None,
                emoji: Some("🎉".to_string()),
            }
        );
    }

    #[test]
    fn text_cap_is_grapheme_chars_not_bytes() {
        // 160 multi-byte chars (each 2 bytes in UTF-8) — well over the byte
        // count but exactly at the char cap.
        let s: String = "ą".repeat(STATUS_TEXT_MAX_CHARS);
        assert!(s.len() > STATUS_TEXT_MAX_CHARS);
        assert_eq!(
            normalize_status(&body(Some(&s), None)),
            NormalizedStatus::Set {
                text: Some(s.clone()),
                emoji: None,
            }
        );

        let too_long: String = "ą".repeat(STATUS_TEXT_MAX_CHARS + 1);
        assert_eq!(
            normalize_status(&body(Some(&too_long), None)),
            NormalizedStatus::InvalidTextTooLong
        );
    }

    #[test]
    fn emoji_cap_is_bytes_not_chars() {
        // ZWJ-joined family sequence is one grapheme cluster but many bytes.
        // Cap is 32 bytes; build something that exceeds it.
        let huge = "👨‍👩‍👧‍👦".repeat(4);
        assert!(huge.len() > STATUS_EMOJI_MAX_BYTES);
        assert_eq!(
            normalize_status(&body(None, Some(&huge))),
            NormalizedStatus::InvalidEmojiTooLong
        );

        let ok = "🙂";
        assert!(ok.len() <= STATUS_EMOJI_MAX_BYTES);
        assert_eq!(
            normalize_status(&body(None, Some(ok))),
            NormalizedStatus::Set {
                text: None,
                emoji: Some(ok.to_string()),
            }
        );
    }

    #[test]
    fn text_validation_runs_before_emoji() {
        // Both invalid → text error reported first; ordering matters for
        // composer UX (text field is the primary input).
        let too_long_text: String = "a".repeat(STATUS_TEXT_MAX_CHARS + 1);
        let too_long_emoji = "👨‍👩‍👧‍👦".repeat(4);
        assert_eq!(
            normalize_status(&body(Some(&too_long_text), Some(&too_long_emoji))),
            NormalizedStatus::InvalidTextTooLong
        );
    }
}
