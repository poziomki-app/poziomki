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
use uuid::Uuid;

use crate::api::state::{
    AttendEventBody, AttendeeStatus, CreateEventBody, DataResponse, SuccessResponse,
    UpdateEventBody,
};
use crate::db::models::events::{Event, NewEvent};
use crate::db::models::profiles::Profile;
use crate::jobs::enqueue_matrix_event_membership_sync;

use super::events_service::{forbidden, load_event, parse_create_dates, require_auth_profile};
use super::events_tags_repo::{sync_event_tags, upsert_attendee};
use super::events_tags_service::{maybe_sync_tags, resolve_event_tag_ids};
use super::events_view::{build_event_response, created_event_response};
use super::events_write_repo;
use super::events_write_service::event_update_inner;

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

async fn finalize_event_create(
    event_id: Uuid,
    event: &Event,
    profile_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Response> {
    if !tag_ids.is_empty() {
        sync_event_tags(event_id, &tag_ids).await?;
    }
    upsert_attendee(event_id, profile_id, "going").await?;
    let data = build_event_response(event, &profile_id).await?;
    Ok(created_event_response(data))
}

pub(in crate::api) async fn event_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let validated = match event_create_validate(&headers, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let (new_event, event_id) = build_create_event(&validated, &payload);

    let inserted = events_write_repo::insert_event(&new_event).await?;

    let tag_ids = resolve_event_tag_ids(payload.tags, payload.tag_ids).await;
    finalize_event_create(event_id, &inserted, validated.profile.id, tag_ids).await
}

pub(in crate::api) async fn event_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let (changeset, event_uuid, profile) = match event_update_inner(&headers, &id, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let updated = events_write_repo::update_event(event_uuid, &changeset).await?;

    maybe_sync_tags(event_uuid, payload.tags, payload.tag_ids).await?;

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

pub(in crate::api) async fn event_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_event, event_uuid) =
        match load_owned_event(&headers, &id, "Only the creator can delete this event").await {
            Ok(data) => data,
            Err(response) => return Ok(*response),
        };

    events_write_repo::delete_event(event_uuid).await?;

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

fn resolve_attend_status(payload: Option<Json<AttendEventBody>>) -> &'static str {
    match payload
        .and_then(|Json(body)| body.status)
        .unwrap_or(AttendeeStatus::Going)
    {
        AttendeeStatus::Going => "going",
        AttendeeStatus::Interested => "interested",
        AttendeeStatus::Invited => "invited",
    }
}

pub(in crate::api) async fn event_attend(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Option<Json<AttendEventBody>>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let status_str = resolve_attend_status(payload);

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

pub(in crate::api) async fn event_leave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_for_leave(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    events_write_repo::delete_event_attendee(event_uuid, profile.id).await?;

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
