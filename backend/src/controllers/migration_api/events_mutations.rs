use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use crate::controllers::migration_api::state::{
    AttendEventBody, AttendeeStatus, CreateEventBody, DataResponse, SuccessResponse,
    UpdateEventBody,
};
use crate::models::_entities::{event_attendees, events, profiles};

use super::events_support::{forbidden, load_event, parse_create_dates, require_auth_profile};
use super::events_tags::{
    maybe_sync_tags, resolve_event_tag_ids, sync_event_tags, upsert_attendee,
};
use super::events_update::event_update_inner;
use super::events_view::{build_event_response, created_event_response};

const fn geo_from_event(event: &events::Model) -> Option<crate::search::GeoPoint> {
    match (event.latitude, event.longitude) {
        (Some(lat), Some(lng)) => Some(crate::search::GeoPoint { lat, lng }),
        _ => None,
    }
}

struct ValidatedCreate {
    profile: profiles::Model,
    title: String,
    starts_at: chrono::DateTime<Utc>,
    ends_at: Option<chrono::DateTime<Utc>>,
}

async fn event_create_validate(
    ctx: &AppContext,
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<ValidatedCreate, Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(&ctx.db, headers).await?;
    let (title, starts_at, ends_at) = parse_create_dates(headers, payload)?;
    Ok(ValidatedCreate {
        profile,
        title,
        starts_at,
        ends_at,
    })
}

fn build_create_event(
    validated: &ValidatedCreate,
    payload: &CreateEventBody,
) -> (events::ActiveModel, Uuid) {
    let now = Utc::now();
    let event_id = Uuid::new_v4();
    let model = events::ActiveModel {
        id: ActiveValue::Set(event_id),
        title: ActiveValue::Set(validated.title.clone()),
        description: ActiveValue::Set(payload.description.clone()),
        cover_image: ActiveValue::Set(payload.cover_image.clone()),
        location: ActiveValue::Set(payload.location.clone()),
        starts_at: ActiveValue::Set(validated.starts_at.into()),
        ends_at: ActiveValue::Set(validated.ends_at.map(Into::into)),
        creator_id: ActiveValue::Set(validated.profile.id),
        conversation_id: ActiveValue::Set(None),
        latitude: ActiveValue::Set(payload.latitude),
        longitude: ActiveValue::Set(payload.longitude),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };
    (model, event_id)
}

pub(in crate::controllers::migration_api) async fn event_create(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let validated = match event_create_validate(&ctx, &headers, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let (model, event_id) = build_create_event(&validated, &payload);
    let inserted = model
        .insert(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let tag_ids = resolve_event_tag_ids(&ctx.db, payload.tags, payload.tag_ids).await;
    if !tag_ids.is_empty() {
        sync_event_tags(&ctx.db, event_id, &tag_ids).await?;
    }

    upsert_attendee(&ctx.db, event_id, validated.profile.id, "going").await?;

    // MEILI_COMPAT_REMOVE
    crate::search::index_event_compat(crate::search::EventDocument {
        id: inserted.id.to_string(),
        title: inserted.title.clone(),
        description: inserted.description.clone(),
        location: inserted.location.clone(),
        starts_at: inserted.starts_at.to_rfc3339(),
        cover_image: inserted.cover_image.clone(),
        creator_name: validated.profile.name.clone(),
        geo: geo_from_event(&inserted),
    });

    let data = build_event_response(&ctx.db, &inserted, &validated.profile.id).await?;
    Ok(created_event_response(data))
}

pub(in crate::controllers::migration_api) async fn event_update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let (active, event_uuid, profile) =
        match event_update_inner(&ctx, &headers, &id, &payload).await {
            Ok(data) => data,
            Err(response) => return Ok(*response),
        };

    let updated = active
        .update(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    maybe_sync_tags(&ctx.db, event_uuid, payload.tags, payload.tag_ids).await?;

    // MEILI_COMPAT_REMOVE
    crate::search::index_event_compat(crate::search::EventDocument {
        id: updated.id.to_string(),
        title: updated.title.clone(),
        description: updated.description.clone(),
        location: updated.location.clone(),
        starts_at: updated.starts_at.to_rfc3339(),
        cover_image: updated.cover_image.clone(),
        creator_name: profile.name.clone(),
        geo: geo_from_event(&updated),
    });

    let data = build_event_response(&ctx.db, &updated, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn load_owned_event(
    ctx: &AppContext,
    headers: &HeaderMap,
    id: &str,
    message: &str,
) -> std::result::Result<(events::Model, Uuid), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(&ctx.db, headers).await?;
    let (event, event_uuid) = load_event(&ctx.db, headers, id).await?;
    if event.creator_id != profile.id {
        return Err(Box::new(forbidden(headers, message)));
    }
    Ok((event, event_uuid))
}

pub(in crate::controllers::migration_api) async fn event_delete(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_event, event_uuid) = match load_owned_event(
        &ctx,
        &headers,
        &id,
        "Only the creator can delete this event",
    )
    .await
    {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    events::Entity::delete_by_id(event_uuid)
        .exec(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    // MEILI_COMPAT_REMOVE
    crate::search::delete_event_compat(event_uuid.to_string());

    Ok(Json(SuccessResponse { success: true }).into_response())
}

async fn load_event_with_profile(
    ctx: &AppContext,
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(events::Model, Uuid, profiles::Model), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(&ctx.db, headers).await?;
    let (event, event_uuid) = load_event(&ctx.db, headers, id).await?;
    Ok((event, event_uuid, profile))
}

pub(in crate::controllers::migration_api) async fn event_attend(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<AttendEventBody>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&ctx, &headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let status_str = match payload.status.unwrap_or(AttendeeStatus::Going) {
        AttendeeStatus::Going => "going",
        AttendeeStatus::Interested => "interested",
        AttendeeStatus::Invited => "invited",
    };

    upsert_attendee(&ctx.db, event_uuid, profile.id, status_str).await?;

    let data = build_event_response(&ctx.db, &event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn load_event_for_leave(
    ctx: &AppContext,
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(events::Model, Uuid, profiles::Model), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(&ctx.db, headers).await?;
    let (event, event_uuid) = load_event(&ctx.db, headers, id).await?;
    if event.creator_id == profile.id {
        return Err(Box::new(forbidden(
            headers,
            "Event creator cannot leave the event",
        )));
    }
    Ok((event, event_uuid, profile))
}

pub(in crate::controllers::migration_api) async fn event_leave(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_for_leave(&ctx, &headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    event_attendees::Entity::delete_many()
        .filter(event_attendees::Column::EventId.eq(event_uuid))
        .filter(event_attendees::Column::ProfileId.eq(profile.id))
        .exec(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let data = build_event_response(&ctx.db, &event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}
