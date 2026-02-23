#[path = "mutations.rs"]
mod events_mutations;
#[path = "support.rs"]
mod events_support;
#[path = "tags.rs"]
mod events_tags;
#[path = "update.rs"]
mod events_update;
#[path = "view.rs"]
mod events_view;

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
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::{DataResponse, EventsQuery};
use crate::db::models::events::Event;
use crate::db::schema::events;
use events_support::{not_found_event, require_auth_profile};
use events_view::attendee_info;

pub(super) use events_mutations::{
    event_attend, event_create, event_delete, event_leave, event_update,
};
pub(super) use events_view::{build_event_response, build_event_responses};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

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

    let mut conn = crate::db::conn().await?;

    let all_events = events::table
        .filter(events::starts_at.ge(now))
        .order(events::starts_at.asc())
        .limit(limit)
        .load::<Event>(&mut conn)
        .await?;

    let data = events_view::build_event_responses(&all_events, &profile.id).await?;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

pub(super) async fn events_mine(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn().await?;

    let my_events = events::table
        .filter(events::creator_id.eq(profile.id))
        .order(events::starts_at.desc())
        .load::<Event>(&mut conn)
        .await?;

    let data = events_view::build_event_responses(&my_events, &profile.id).await?;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
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

    let mut conn = crate::db::conn().await?;

    let Some(event) = events::table
        .find(event_uuid)
        .first::<Event>(&mut conn)
        .await
        .optional()?
    else {
        return Ok(not_found_event(&headers, &id));
    };

    let data = build_event_response(&event, &profile.id).await?;
    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
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

    let mut conn = crate::db::conn().await?;

    let exists = events::table
        .find(event_uuid)
        .first::<Event>(&mut conn)
        .await
        .optional()?
        .is_some();

    if !exists {
        return Ok(not_found_event(&headers, &id));
    }

    let data = attendee_info(event_uuid).await?;
    Ok(Json(DataResponse { data }).into_response())
}
