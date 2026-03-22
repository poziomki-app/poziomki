type Result<T> = crate::error::AppResult<T>;

use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};

use crate::api::state::{ReportEventBody, SuccessResponse};
use crate::app::AppContext;

use super::events_service::{
    forbidden, load_event, not_found_event, require_auth_profile, validation_error,
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

pub(in crate::api) async fn event_report(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReportEventBody>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let (event, event_id) = match load_event(&headers, &id).await {
        Ok(ev) => ev,
        Err(response) => return Ok(*response),
    };

    if event.creator_id == profile.id {
        return Ok(forbidden(
            &headers,
            "Nie możesz zgłosić własnego wydarzenia",
        ));
    }

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

    let Some(inserted) =
        report_repo::insert_event_report(profile.id, event_id, reason, description).await?
    else {
        return Ok(not_found_event(&headers, &id));
    };

    if !inserted {
        return Ok(validation_error(
            &headers,
            "To wydarzenie zostało już przez Ciebie zgłoszone",
        ));
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}
