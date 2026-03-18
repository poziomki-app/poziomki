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

use super::events_service::{forbidden, load_event, require_auth_profile, validation_error};
use super::report_repo;

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
        return Ok(forbidden(&headers, "You cannot report your own event"));
    }

    let reason = body.reason.trim().to_string();
    if reason.is_empty() {
        return Ok(validation_error(&headers, "Reason is required"));
    }
    if reason.chars().count() > 2000 {
        return Ok(validation_error(
            &headers,
            "Reason must be at most 2000 characters",
        ));
    }

    let inserted = report_repo::insert_event_report(profile.id, event_id, reason).await?;

    if !inserted {
        return Ok(validation_error(
            &headers,
            "You have already reported this event",
        ));
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}
