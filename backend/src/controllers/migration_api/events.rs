#[path = "events_mutations.rs"]
mod events_mutations;
#[path = "events_support.rs"]
mod events_support;
#[path = "events_tags.rs"]
mod events_tags;
#[path = "events_update.rs"]
mod events_update;
#[path = "events_view.rs"]
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
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::state::{DataResponse, EventsQuery};
use crate::models::_entities::events;
use events_support::{not_found_event, require_auth_profile};
use events_view::attendee_info;

pub(super) use events_mutations::{
    event_attend, event_create, event_delete, event_leave, event_update,
};
pub(super) use events_view::{build_event_response, build_event_responses};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

pub(super) async fn events_list(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = u64::from(query.limit.unwrap_or(20).clamp(1, 100));
    let now = Utc::now();

    let all_events = events::Entity::find()
        .filter(events::Column::StartsAt.gte(now))
        .order_by_asc(events::Column::StartsAt)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let data = events_view::build_event_responses(&ctx.db, &all_events, &profile.id).await?;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

pub(super) async fn events_mine(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let my_events = events::Entity::find()
        .filter(events::Column::CreatorId.eq(profile.id))
        .order_by_desc(events::Column::StartsAt)
        .all(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let data = events_view::build_event_responses(&ctx.db, &my_events, &profile.id).await?;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

pub(super) async fn event_get(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let Some(event) = events::Entity::find_by_id(event_uuid)
        .one(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?
    else {
        return Ok(not_found_event(&headers, &id));
    };

    let data = build_event_response(&ctx.db, &event, &profile.id).await?;
    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

pub(super) async fn event_attendees(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_profile, _user_pid) = match require_auth_profile(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let exists = events::Entity::find_by_id(event_uuid)
        .one(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?
        .is_some();

    if !exists {
        return Ok(not_found_event(&headers, &id));
    }

    let data = attendee_info(&ctx.db, event_uuid).await?;
    Ok(Json(DataResponse { data }).into_response())
}
