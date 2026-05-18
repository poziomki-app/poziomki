//! Test-phase feedback collection. Mobile clients POST a 1–5 star
//! rating plus an optional free-form message; we persist it for triage
//! during the closed test rollout.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

use super::state::DataResponse;
use crate::api::common::{auth_or_respond, error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db::models::user_feedback::NewUserFeedback;
use crate::db::schema::user_feedback;

type Result<T> = crate::error::AppResult<T>;

const MAX_MESSAGE_LEN: usize = 4000;
const MAX_APP_VERSION_LEN: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreateFeedbackBody {
    pub rating: i16,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub app_version: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct CreateFeedbackResponse {
    id: Uuid,
}

pub(super) async fn create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<CreateFeedbackBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if !(1..=5).contains(&body.rating) {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "rating must be between 1 and 5".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let message = body
        .message
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_MESSAGE_LEN).collect::<String>());

    let app_version = body
        .app_version
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_APP_VERSION_LEN).collect::<String>());

    let row = NewUserFeedback {
        id: Uuid::new_v4(),
        user_id: user.id,
        rating: body.rating,
        message,
        app_version,
        created_at: Utc::now(),
    };

    let mut conn = crate::db::conn().await?;
    diesel::insert_into(user_feedback::table)
        .values(&row)
        .execute(&mut conn)
        .await?;

    Ok(Json(DataResponse {
        data: CreateFeedbackResponse { id: row.id },
    })
    .into_response())
}
