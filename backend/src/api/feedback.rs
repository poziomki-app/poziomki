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
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

use super::state::DataResponse;
use crate::api::common::{auth_or_respond, error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;
use crate::db::models::user_feedback::NewUserFeedback;
use crate::db::schema::user_feedback;

type Result<T> = crate::error::AppResult<T>;

const MAX_MESSAGE_LEN: usize = 4000;
const MAX_APP_VERSION_LEN: usize = 64;
const FEEDBACK_NOTIFY_TO: &str = "kontakt@poziomki.app";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreateFeedbackBody {
    pub rating: i16,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub feature_request: Option<String>,
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

    let feature_request = body
        .feature_request
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_MESSAGE_LEN).collect::<String>());

    let row = NewUserFeedback {
        id: Uuid::new_v4(),
        user_id: user.id,
        rating: body.rating,
        message: message.clone(),
        app_version: app_version.clone(),
        created_at: Utc::now(),
        feature_request: feature_request.clone(),
    };

    // user_feedback is RLS-gated by user_id = app.current_user_id(). The
    // viewer-context wrapper sets that for the duration of the tx; a
    // plain db::conn() insert has no viewer and hits a WITH CHECK
    // violation (the bug that was masking submissions before).
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let insert_row = row.clone();
    db::with_viewer_tx(viewer, move |conn| {
        async move {
            diesel::insert_into(user_feedback::table)
                .values(&insert_row)
                .execute(conn)
                .await?;
            Ok::<_, diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    // Notify the operator inbox in the background. Failure to send mail
    // must never make the user think their submission was lost — they
    // already saw a 200. The body is anonymous: rating + free text only,
    // no user id or email. Look up the row by id in the DB if needed.
    let rating = body.rating;
    let app_version_for_mail = app_version.clone();
    tokio::spawn(async move {
        if let Err(e) = send_feedback_email(
            rating,
            message.as_deref(),
            feature_request.as_deref(),
            app_version_for_mail.as_deref(),
        )
        .await
        {
            tracing::warn!(error = %e, "feedback notify email failed");
        }
    });

    Ok(Json(DataResponse {
        data: CreateFeedbackResponse { id: row.id },
    })
    .into_response())
}

/// Send the operator-facing feedback notification to kontakt@poziomki.app.
/// Anonymous: just rating + free text + app version.
async fn send_feedback_email(
    rating: i16,
    message: Option<&str>,
    feature_request: Option<&str>,
    app_version: Option<&str>,
) -> std::result::Result<(), String> {
    use std::fmt::Write;

    let stars: String = (1..=5)
        .map(|i| if i <= rating { '★' } else { '☆' })
        .collect();
    let subject = format!("Nowa opinia {stars} ({rating}/5)");

    let mut body = String::new();
    let _ = writeln!(body, "Ocena: {stars} ({rating}/5)");
    if let Some(v) = app_version {
        let _ = writeln!(body, "Wersja: {v}");
    }
    body.push_str("\nWiadomość:\n");
    body.push_str(message.unwrap_or("(brak)"));
    body.push_str("\n\nCo dodać do apki:\n");
    body.push_str(feature_request.unwrap_or("(brak)"));
    body.push('\n');

    crate::api::auth::send_simple_mail(FEEDBACK_NOTIFY_TO, &subject, &body).await
}
