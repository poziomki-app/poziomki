type Result<T> = crate::error::AppResult<T>;

use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use diesel_async::scoped_futures::ScopedFutureExt;

use crate::api::state::{ReportEventBody, SuccessResponse};
use crate::app::AppContext;
use crate::db;

use super::events_service::{
    forbidden, load_event_by_id, load_profile_for_user, not_found_event, profile_not_found,
    validation_error,
};
use super::report_repo;

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

enum ReportOutcome {
    NoProfile,
    NotFound,
    OwnEvent,
    Duplicate,
    Inserted,
}

fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

pub(in crate::api) async fn event_report(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReportEventBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let reason = body.reason.trim().to_lowercase();
    if !VALID_REASONS.contains(&reason.as_str()) {
        return Ok(validation_error(&headers, "Nieprawidłowy powód zgłoszenia"));
    }

    let description = body
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    if let Some(ref desc) = description {
        if desc.chars().count() > 2000 {
            return Ok(validation_error(
                &headers,
                "Opis może mieć maksymalnie 2000 znaków",
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
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<ReportOutcome, diesel::result::Error>(ReportOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(ReportOutcome::NotFound);
            };
            if event.creator_id == profile.id {
                return Ok(ReportOutcome::OwnEvent);
            }
            match report_repo::insert_event_report(
                conn,
                profile.id,
                event_uuid,
                reason,
                description,
            )
            .await
            .map_err(into_diesel)?
            {
                None => Ok(ReportOutcome::NotFound),
                Some(false) => Ok(ReportOutcome::Duplicate),
                Some(true) => Ok(ReportOutcome::Inserted),
            }
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        ReportOutcome::NoProfile => Ok(profile_not_found(&headers)),
        ReportOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        ReportOutcome::OwnEvent => Ok(forbidden(
            &headers,
            "Nie możesz zgłosić własnego wydarzenia",
        )),
        ReportOutcome::Duplicate => Ok(validation_error(
            &headers,
            "To wydarzenie zostało już przez Ciebie zgłoszone",
        )),
        ReportOutcome::Inserted => Ok(Json(SuccessResponse { success: true }).into_response()),
    }
}
