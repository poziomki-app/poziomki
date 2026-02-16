use axum::http::HeaderMap;
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::controllers::migration_api::state::UpdateEventBody;
use crate::models::_entities::{events, profiles};

use super::events_support::{
    self, forbidden, load_event, require_auth_profile, validate_event_description,
    validate_event_location,
};

type EventDates = (chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>);

fn validate_update_title(value: &str) -> std::result::Result<(), &'static str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 200 {
        Err("Title must be between 1 and 200 characters")
    } else {
        Ok(())
    }
}

fn validate_event_basic_fields(
    headers: &HeaderMap,
    payload: &UpdateEventBody,
) -> std::result::Result<(), Box<Response>> {
    let val_err = |msg| Box::new(events_support::validation_error(headers, msg));

    payload
        .title
        .as_deref()
        .map(validate_update_title)
        .transpose()
        .map_err(val_err)?;
    validate_event_description(payload.description.as_ref().and_then(|d| d.as_ref()))
        .map_err(val_err)?;
    validate_event_location(payload.location.as_ref().and_then(|l| l.as_ref())).map_err(val_err)?;

    Ok(())
}

fn apply_basic_fields(active: &mut events::ActiveModel, payload: &UpdateEventBody) {
    if let Some(title) = &payload.title {
        active.title = ActiveValue::Set(title.trim().to_string());
    }

    if let Some(desc) = &payload.description {
        active.description = ActiveValue::Set(desc.clone());
    }

    if let Some(loc) = &payload.location {
        active.location = ActiveValue::Set(loc.clone());
    }

    if let Some(cover) = &payload.cover_image {
        active.cover_image = ActiveValue::Set(cover.clone());
    }

    if let Some(lat) = &payload.latitude {
        active.latitude = ActiveValue::Set(*lat);
    }
    if let Some(lng) = &payload.longitude {
        active.longitude = ActiveValue::Set(*lng);
    }
}

fn parse_optional_ts(
    headers: &HeaderMap,
    raw: &str,
) -> std::result::Result<chrono::DateTime<Utc>, Box<Response>> {
    events_support::parse_timestamp(raw)
        .map_err(|msg| Box::new(events_support::validation_error(headers, msg)))
}

fn parse_event_dates(
    headers: &HeaderMap,
    event: &events::Model,
    payload: &UpdateEventBody,
) -> std::result::Result<EventDates, Box<Response>> {
    let starts = payload
        .starts_at
        .as_deref()
        .map(|s| parse_optional_ts(headers, s))
        .transpose()?
        .unwrap_or_else(|| event.starts_at.with_timezone(&Utc));

    let current_ends = event.ends_at.map(|e| e.with_timezone(&Utc));
    let ends = match payload.ends_at.as_ref() {
        Some(Some(s)) => Some(parse_optional_ts(headers, s)?),
        Some(None) => None,
        None => current_ends,
    };

    if ends.is_some_and(|end| end <= starts) {
        return Err(Box::new(events_support::validation_error(
            headers,
            "Event end time must be after start time",
        )));
    }

    Ok((starts, ends))
}

fn apply_update_fields(
    event: &events::Model,
    payload: &UpdateEventBody,
    dates: EventDates,
) -> events::ActiveModel {
    let mut active: events::ActiveModel = event.clone().into();

    apply_basic_fields(&mut active, payload);

    let (new_starts, new_ends) = dates;

    active.starts_at = ActiveValue::Set(new_starts.into());
    active.ends_at = ActiveValue::Set(new_ends.map(Into::into));
    active.updated_at = ActiveValue::Set(Utc::now().into());

    active
}

fn validate_creator(
    headers: &HeaderMap,
    event: &events::Model,
    profile_id: Uuid,
) -> std::result::Result<(), Box<Response>> {
    if event.creator_id != profile_id {
        return Err(Box::new(forbidden(
            headers,
            "Only the creator can update this event",
        )));
    }
    Ok(())
}

pub(in crate::controllers::migration_api) async fn event_update_inner(
    ctx: &AppContext,
    headers: &HeaderMap,
    id: &str,
    payload: &UpdateEventBody,
) -> std::result::Result<(events::ActiveModel, Uuid, profiles::Model), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(&ctx.db, headers).await?;
    let (event, event_uuid) = load_event(&ctx.db, headers, id).await?;
    validate_creator(headers, &event, profile.id)?;
    validate_event_basic_fields(headers, payload)?;
    let dates = parse_event_dates(headers, &event, payload)?;
    let active = apply_update_fields(&event, payload, dates);
    Ok((active, event_uuid, profile))
}
