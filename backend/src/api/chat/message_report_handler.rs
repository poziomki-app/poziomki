//! Per-message moderation report endpoint.
//!
//! Conversation-level reports already exist; this is the message-
//! granular variant. Typical flow: Bielik-Guard flags a chat message,
//! the recipient blurs it, taps to reveal, then taps the floating
//! flag → this endpoint. We snapshot the auto-mod verdict +
//! categories at file time so admin tooling can correlate the report
//! to the model's opinion (false positive vs. true positive that the
//! model also caught).
//!
//! `(message_id, reporter_user_id)` is the natural PK; reporting twice
//! is idempotent.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::api::state::{DataResponse, SuccessResponse};
use crate::app::AppContext;
use crate::db;
use crate::db::schema::{chat_message_reports::dsl as r, messages::dsl as m};

type Result<T> = crate::error::AppResult<T>;

/// Mirrors the conversation-report reasons so the same picker UI
/// in the mobile app can drive both endpoints.
const VALID_REASONS: &[&str] = &[
    "spam",
    "inappropriate",
    "misleading",
    "harassment",
    "hate_speech",
    "violence",
    "scam",
    "other",
];

#[derive(Clone, Debug, Deserialize)]
pub struct ReportMessageBody {
    pub reason: String,
    #[serde(default)]
    pub description: Option<String>,
}

enum ReportOutcome {
    NotFound,
    RateLimited,
    Inserted,
}

/// Cap on new reports per reporter per rolling 24h. Idempotent re-reports
/// of the same message don't count (they're absorbed by the PK conflict),
/// so this only bites someone trying to flood the moderation queue.
const REPORTS_PER_DAY: i64 = 30;

pub(in crate::api) async fn message_report(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReportMessageBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let message_id = match parse_uuid_response(&id, "message", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let reason = body.reason.trim().to_lowercase();
    if !VALID_REASONS.contains(&reason.as_str()) {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Nieprawidłowy powód zgłoszenia".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let description = body
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    if let Some(ref desc) = description {
        if desc.chars().count() > 2000 {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                &headers,
                ErrorSpec {
                    error: "Opis może mieć maksymalnie 2000 znaków".to_string(),
                    code: "VALIDATION_ERROR",
                    details: None,
                },
            ));
        }
    }

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let reporter_user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            // Snapshot the moderation context as it stands now. Even
            // if the worker rescans later (it won't today), the
            // report row keeps what the user saw at file time.
            let snapshot: Option<(Option<String>, Vec<String>)> = m::messages
                .filter(m::id.eq(message_id))
                .select((m::moderation_verdict, m::moderation_categories))
                .first::<(Option<String>, Vec<String>)>(conn)
                .await
                .optional()?;
            let Some((automod_verdict, automod_categories)) = snapshot else {
                return Ok::<_, diesel::result::Error>(ReportOutcome::NotFound);
            };

            let day_ago = chrono::Utc::now() - chrono::Duration::hours(24);
            let recent: i64 = r::chat_message_reports
                .filter(r::reporter_user_id.eq(reporter_user_id))
                .filter(r::created_at.gt(day_ago))
                .count()
                .get_result(conn)
                .await?;
            if recent >= REPORTS_PER_DAY {
                return Ok(ReportOutcome::RateLimited);
            }

            diesel::insert_into(r::chat_message_reports)
                .values((
                    r::message_id.eq(message_id),
                    r::reporter_user_id.eq(reporter_user_id),
                    r::reason.eq(&reason),
                    r::description.eq(description.as_deref()),
                    r::automoderation_verdict.eq(automod_verdict.as_deref()),
                    r::automoderation_categories.eq(&automod_categories),
                    r::status.eq("open"),
                    r::created_at.eq(chrono::Utc::now()),
                ))
                .on_conflict((r::message_id, r::reporter_user_id))
                .do_nothing()
                .execute(conn)
                .await?;

            Ok(ReportOutcome::Inserted)
        }
        .scope_boxed()
    })
    .await?;

    match outcome {
        ReportOutcome::NotFound => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Message not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        ReportOutcome::RateLimited => Ok(error_response(
            StatusCode::TOO_MANY_REQUESTS,
            &headers,
            ErrorSpec {
                error: "Osiągnięto dzienny limit zgłoszeń. Spróbuj ponownie za 24 godziny."
                    .to_string(),
                code: "RATE_LIMITED",
                details: None,
            },
        )),
        ReportOutcome::Inserted => Ok(Json(DataResponse {
            data: SuccessResponse { success: true },
        })
        .into_response()),
    }
}
