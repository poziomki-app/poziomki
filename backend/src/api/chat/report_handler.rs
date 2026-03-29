use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::{Path, State},
    Json,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::api::state::SuccessResponse;
use crate::app::AppContext;
use crate::db::schema::profiles;

use super::report_repo;

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

    // Resolve reporter's profile id
    let mut conn = crate::db::conn().await?;

    let reporter_profile_id: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .select(profiles::id)
        .first(&mut conn)
        .await
        .optional()?;

    let Some(reporter_profile_id) = reporter_profile_id else {
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

    // Verify reporter is a member
    if !super::conversations::is_member(conversation_id, user.id).await? {
        return Ok(error_response(
            StatusCode::FORBIDDEN,
            &headers,
            ErrorSpec {
                error: "Musisz być członkiem tej rozmowy".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        ));
    }

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

    let Some(inserted) = report_repo::insert_conversation_report(
        reporter_profile_id,
        conversation_id,
        reason,
        description,
    )
    .await?
    else {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Conversation not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    if !inserted {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Ta rozmowa została już przez Ciebie zgłoszona".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}
