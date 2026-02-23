use axum::http::HeaderMap;
use chrono::Utc;
use uuid::Uuid;

use crate::api::state::UpdateEventBody;
use crate::db::models::events::{Event, EventChangeset};
use crate::db::models::profiles::Profile;

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
) -> std::result::Result<(), Box<axum::response::Response>> {
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

fn parse_optional_ts(
    headers: &HeaderMap,
    raw: &str,
) -> std::result::Result<chrono::DateTime<Utc>, Box<axum::response::Response>> {
    events_support::parse_timestamp(raw)
        .map_err(|msg| Box::new(events_support::validation_error(headers, msg)))
}

fn parse_event_dates(
    headers: &HeaderMap,
    event: &Event,
    payload: &UpdateEventBody,
) -> std::result::Result<EventDates, Box<axum::response::Response>> {
    let starts = payload
        .starts_at
        .as_deref()
        .map(|s| parse_optional_ts(headers, s))
        .transpose()?
        .unwrap_or(event.starts_at);

    let current_ends = event.ends_at;
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

fn build_update_changeset(payload: &UpdateEventBody, dates: EventDates) -> EventChangeset {
    let mut changeset = EventChangeset::default();

    if let Some(title) = &payload.title {
        changeset.title = Some(title.trim().to_string());
    }
    if let Some(desc) = &payload.description {
        changeset.description = Some(desc.clone());
    }
    if let Some(loc) = &payload.location {
        changeset.location = Some(loc.clone());
    }
    if let Some(cover) = &payload.cover_image {
        changeset.cover_image = Some(cover.clone());
    }
    if let Some(lat) = &payload.latitude {
        changeset.latitude = Some(*lat);
    }
    if let Some(lng) = &payload.longitude {
        changeset.longitude = Some(*lng);
    }

    let (new_starts, new_ends) = dates;
    changeset.starts_at = Some(new_starts);
    changeset.ends_at = Some(new_ends);
    changeset.updated_at = Some(Utc::now());

    changeset
}

fn validate_creator(
    headers: &HeaderMap,
    event: &Event,
    profile_id: Uuid,
) -> std::result::Result<(), Box<axum::response::Response>> {
    if event.creator_id != profile_id {
        return Err(Box::new(forbidden(
            headers,
            "Only the creator can update this event",
        )));
    }
    Ok(())
}

pub(in crate::api) async fn event_update_inner(
    headers: &HeaderMap,
    id: &str,
    payload: &UpdateEventBody,
) -> std::result::Result<(EventChangeset, Uuid, Profile), Box<axum::response::Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    validate_creator(headers, &event, profile.id)?;
    validate_event_basic_fields(headers, payload)?;
    let dates = parse_event_dates(headers, &event, payload)?;
    let changeset = build_update_changeset(payload, dates);
    Ok((changeset, event_uuid, profile))
}
