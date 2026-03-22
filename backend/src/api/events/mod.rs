#[path = "interactions_repo.rs"]
mod events_interactions_repo;
#[path = "repo.rs"]
mod events_repo;
#[path = "service.rs"]
mod events_service;
#[path = "tags_repo.rs"]
mod events_tags_repo;
#[path = "tags_service.rs"]
mod events_tags_service;
#[path = "view.rs"]
mod events_view;
#[path = "write_handler.rs"]
mod events_write_handler;
#[path = "write_repo.rs"]
mod events_write_repo;
#[path = "write_service.rs"]
mod events_write_service;
mod report_handler;
mod report_repo;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use super::state::{DataResponse, EventsQuery};
use events_service::{not_found_event, require_auth_profile};
use events_view::attendee_info;

pub(super) use events_interactions_repo::EVENT_INTERACTION_SAVED;
pub(super) use events_view::{build_event_response, build_event_responses_with_conn};
pub(super) use events_write_handler::{
    event_approve_attendee, event_attend, event_create, event_delete, event_leave,
    event_reject_attendee, event_save, event_unsave, event_update,
};
pub(super) use report_handler::event_report;

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

fn with_private_short_cache(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    response
}

pub(super) async fn events_list(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = i64::from(query.limit.unwrap_or(20).clamp(1, 100));
    let now = Utc::now();

    let all_events = events_repo::list_upcoming_events(now, limit).await?;

    let data = events_view::build_event_responses(&all_events, &profile.id).await?;
    Ok(with_private_short_cache(
        Json(DataResponse { data }).into_response(),
    ))
}

pub(super) async fn events_mine(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let my_events = events_repo::list_events_by_creator(profile.id).await?;

    let data = events_view::build_event_responses(&my_events, &profile.id).await?;
    Ok(with_private_short_cache(
        Json(DataResponse { data }).into_response(),
    ))
}

pub(super) async fn events_saved(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let saved_events = events_repo::list_saved_events(profile.id).await?;

    let data = events_view::build_event_responses(&saved_events, &profile.id).await?;
    Ok(with_private_short_cache(
        Json(DataResponse { data }).into_response(),
    ))
}

pub(super) async fn event_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let Some(event) = events_repo::find_event(event_uuid).await? else {
        return Ok(not_found_event(&headers, &id));
    };

    let data = build_event_response(&event, &profile.id).await?;
    Ok(with_private_short_cache(
        Json(DataResponse { data }).into_response(),
    ))
}

pub(super) async fn event_attendees(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let Some(event) = events_repo::find_event(event_uuid).await? else {
        return Ok(not_found_event(&headers, &id));
    };

    let data = attendee_info(event_uuid, event.creator_id).await?;
    Ok(Json(DataResponse { data }).into_response())
}
