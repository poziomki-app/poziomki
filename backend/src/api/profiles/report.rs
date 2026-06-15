use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::state::{DataResponse, SuccessResponse};
use crate::api::{error_response, ErrorSpec};
use crate::db::models::reports::NewReport;
use crate::db::schema::{profiles, reports};

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
const MAX_DESCRIPTION_LEN: usize = 2000;

pub(in crate::api) async fn profile_report(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    my_profile_id: Uuid,
    target_id: Uuid,
    reason: String,
    description: Option<String>,
) -> crate::error::AppResult<Response> {
    if my_profile_id == target_id {
        return Ok(validation_error(headers, "Nie możesz zgłosić siebie"));
    }
    if !VALID_REASONS.contains(&reason.as_str()) {
        return Ok(validation_error(headers, "Nieprawidłowy powód zgłoszenia"));
    }
    if description
        .as_ref()
        .is_some_and(|d| d.len() > MAX_DESCRIPTION_LEN)
    {
        return Ok(validation_error(headers, "Opis jest zbyt długi"));
    }

    let target_exists = profiles::table
        .find(target_id)
        .select(profiles::id)
        .first::<Uuid>(conn)
        .await
        .optional()?;
    if target_exists.is_none() {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: "Profile not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    let new = NewReport {
        reporter_id: my_profile_id,
        target_type: "profile".to_string(),
        target_id,
        reason,
        description,
    };
    diesel::insert_into(reports::table)
        .values(&new)
        .on_conflict_do_nothing()
        .execute(conn)
        .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

fn validation_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}
