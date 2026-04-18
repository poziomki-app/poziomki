use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{Path, State},
    Json,
};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::api::state::SuccessResponse;
use crate::app::AppContext;
use crate::db;
use crate::db::schema::profiles;

use super::{conversations, report_repo};

type Result<T> = crate::error::AppResult<T>;

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
pub struct ReportConversationBody {
    pub reason: String,
    #[serde(default)]
    pub description: Option<String>,
}

enum ReportOutcome {
    NoProfile,
    NotMember,
    ConversationMissing,
    Duplicate,
    Inserted,
}

pub(in crate::api) async fn conversation_report(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReportConversationBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let conversation_id = match parse_uuid_response(&id, "conversation", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    // Validate input before touching the DB.
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
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let reporter_profile_id: Option<uuid::Uuid> = profiles::table
                .filter(profiles::user_id.eq(user_id))
                .select(profiles::id)
                .first(conn)
                .await
                .optional()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let Some(reporter_profile_id) = reporter_profile_id else {
                return Ok::<_, diesel::result::Error>(ReportOutcome::NoProfile);
            };

            if !conversations::is_member(conn, conversation_id, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            {
                return Ok(ReportOutcome::NotMember);
            }

            match report_repo::insert_conversation_report(
                conn,
                reporter_profile_id,
                conversation_id,
                reason,
                description,
            )
            .await
            .map_err(|_| diesel::result::Error::RollbackTransaction)?
            {
                None => Ok(ReportOutcome::ConversationMissing),
                Some(false) => Ok(ReportOutcome::Duplicate),
                Some(true) => Ok(ReportOutcome::Inserted),
            }
        }
        .scope_boxed()
    })
    .await?;

    match outcome {
        ReportOutcome::NoProfile | ReportOutcome::NotMember => Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: if matches!(outcome, ReportOutcome::NoProfile) {
                    "Profile required".to_string()
                } else {
                    "Musisz być członkiem tej rozmowy".to_string()
                },
                code: "FORBIDDEN",
                details: None,
            },
        )),
        ReportOutcome::ConversationMissing => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Conversation not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        ReportOutcome::Duplicate => Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Ta rozmowa została już przez Ciebie zgłoszona".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        )),
        ReportOutcome::Inserted => Ok(Json(SuccessResponse { success: true }).into_response()),
    }
}
