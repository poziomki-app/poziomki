type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::controllers::migration_api::state::{
    AttendEventBody, AttendeeStatus, CreateEventBody, DataResponse, SuccessResponse,
    UpdateEventBody,
};
use crate::db::models::events::{Event, NewEvent};
use crate::db::models::profiles::Profile;
use crate::db::schema::{event_attendees, events};
use crate::tasks::enqueue_matrix_event_membership_sync;

use super::events_support::{forbidden, load_event, parse_create_dates, require_auth_profile};
use super::events_tags::{
    maybe_sync_tags, resolve_event_tag_ids, sync_event_tags, upsert_attendee,
};
use super::events_update::event_update_inner;
use super::events_view::{build_event_response, created_event_response};

struct ValidatedCreate {
    profile: Profile,
    title: String,
    starts_at: chrono::DateTime<Utc>,
    ends_at: Option<chrono::DateTime<Utc>>,
}

async fn event_create_validate(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<ValidatedCreate, Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (title, starts_at, ends_at) = parse_create_dates(headers, payload)?;
    Ok(ValidatedCreate {
        profile,
        title,
        starts_at,
        ends_at,
    })
}

fn build_create_event(validated: &ValidatedCreate, payload: &CreateEventBody) -> (NewEvent, Uuid) {
    let now = Utc::now();
    let event_id = Uuid::new_v4();
    let model = NewEvent {
        id: event_id,
        title: validated.title.clone(),
        description: payload.description.clone(),
        cover_image: payload.cover_image.clone(),
        location: payload.location.clone(),
        starts_at: validated.starts_at,
        ends_at: validated.ends_at,
        creator_id: validated.profile.id,
        conversation_id: None,
        latitude: payload.latitude,
        longitude: payload.longitude,
        created_at: now,
        updated_at: now,
    };
    (model, event_id)
}

pub(in crate::controllers::migration_api) async fn event_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let validated = match event_create_validate(&headers, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let (new_event, event_id) = build_create_event(&validated, &payload);

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let inserted = diesel::insert_into(events::table)
        .values(&new_event)
        .get_result::<Event>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_ids = resolve_event_tag_ids(payload.tags, payload.tag_ids).await;
    if !tag_ids.is_empty() {
        sync_event_tags(event_id, &tag_ids).await?;
    }

    upsert_attendee(event_id, validated.profile.id, "going").await?;

    crate::search::invalidate_search_cache();

    let data = build_event_response(&inserted, &validated.profile.id).await?;
    Ok(created_event_response(data))
}

pub(in crate::controllers::migration_api) async fn event_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let (changeset, event_uuid, profile) = match event_update_inner(&headers, &id, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let updated = diesel::update(events::table.find(event_uuid))
        .set(&changeset)
        .get_result::<Event>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    maybe_sync_tags(event_uuid, payload.tags, payload.tag_ids).await?;

    crate::search::invalidate_search_cache();

    let data = build_event_response(&updated, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn load_owned_event(
    headers: &HeaderMap,
    id: &str,
    message: &str,
) -> std::result::Result<(Event, Uuid), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    if event.creator_id != profile.id {
        return Err(Box::new(forbidden(headers, message)));
    }
    Ok((event, event_uuid))
}

pub(in crate::controllers::migration_api) async fn event_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_event, event_uuid) =
        match load_owned_event(&headers, &id, "Only the creator can delete this event").await {
            Ok(data) => data,
            Err(response) => return Ok(*response),
        };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    diesel::delete(events::table.find(event_uuid))
        .execute(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    crate::search::invalidate_search_cache();

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

async fn load_event_with_profile(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Event, Uuid, Profile), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    Ok((event, event_uuid, profile))
}

pub(in crate::controllers::migration_api) async fn event_attend(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Option<Json<AttendEventBody>>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let status_str = match payload
        .and_then(|Json(body)| body.status)
        .unwrap_or(AttendeeStatus::Going)
    {
        AttendeeStatus::Going => "going",
        AttendeeStatus::Interested => "interested",
        AttendeeStatus::Invited => "invited",
    };

    upsert_attendee(event_uuid, profile.id, status_str).await?;
    if let Err(error) = enqueue_matrix_event_membership_sync(&event.id, &profile.id, false).await {
        tracing::warn!(
            %error,
            event_id = %event.id,
            profile_id = %profile.id,
            "failed to enqueue matrix membership sync after attend"
        );
    }

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn load_event_for_leave(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Event, Uuid, Profile), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    if event.creator_id == profile.id {
        return Err(Box::new(forbidden(
            headers,
            "Event creator cannot leave the event",
        )));
    }
    Ok((event, event_uuid, profile))
}

pub(in crate::controllers::migration_api) async fn event_leave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_for_leave(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_uuid))
            .filter(event_attendees::profile_id.eq(profile.id)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| crate::error::AppError::Any(e.into()))?;

    if let Err(error) = enqueue_matrix_event_membership_sync(&event.id, &profile.id, true).await {
        tracing::warn!(
            %error,
            event_id = %event.id,
            profile_id = %profile.id,
            "failed to enqueue matrix membership sync after leave"
        );
    }

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}
