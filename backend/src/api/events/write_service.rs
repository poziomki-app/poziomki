use axum::http::HeaderMap;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::extract_filename;
use crate::api::state::UpdateEventBody;
use crate::db::models::events::{Event, EventChangeset};
use crate::db::schema::uploads;

use super::events_service::{
    self, forbidden, validate_event_category, validate_event_description, validate_event_location,
    validate_max_attendees,
};

/// Verify that `filename` is a non-deleted upload owned by `profile_id`.
/// Without this, a creator could set `cover_image` to any filename they
/// observed (e.g. extracted from a public profile/event URL) and the
/// API would issue signed URLs pointing at the original owner's bytes.
pub(in crate::api) async fn verify_event_cover_ownership(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    profile_id: Uuid,
    raw: &str,
) -> std::result::Result<String, Box<axum::response::Response>> {
    let filename = extract_filename(raw);
    let owned: Option<String> = uploads::table
        .filter(uploads::owner_id.eq(Some(profile_id)))
        .filter(uploads::filename.eq(&filename))
        .filter(uploads::deleted.eq(false))
        .select(uploads::filename)
        .first::<String>(conn)
        .await
        .optional()
        .map_err(|_| {
            Box::new(events_service::validation_error(
                headers,
                "Upload storage is temporarily unavailable",
            ))
        })?;
    if owned.is_none() {
        return Err(Box::new(events_service::validation_error(
            headers,
            "Cover image must reference your uploaded file",
        )));
    }
    Ok(filename)
}

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
    let val_err = |msg| Box::new(events_service::validation_error(headers, msg));

    payload
        .title
        .as_deref()
        .map(validate_update_title)
        .transpose()
        .map_err(val_err)?;
    validate_event_description(payload.description.as_ref().and_then(|d| d.as_ref()))
        .map_err(val_err)?;
    validate_event_category(payload.category.as_ref().and_then(|c| c.as_ref())).map_err(val_err)?;
    validate_event_location(payload.location.as_ref().and_then(|l| l.as_ref())).map_err(val_err)?;
    validate_max_attendees(payload.max_attendees.flatten()).map_err(val_err)?;

    Ok(())
}

fn parse_optional_ts(
    headers: &HeaderMap,
    raw: &str,
) -> std::result::Result<chrono::DateTime<Utc>, Box<axum::response::Response>> {
    events_service::parse_timestamp(raw)
        .map_err(|msg| Box::new(events_service::validation_error(headers, msg)))
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
        return Err(Box::new(events_service::validation_error(
            headers,
            "Event end time must be after start time",
        )));
    }

    Ok((starts, ends))
}

fn build_update_changeset(payload: &UpdateEventBody, dates: EventDates) -> EventChangeset {
    let mut changeset = EventChangeset::default();

    if let Some(req_approval) = payload.requires_approval {
        changeset.requires_approval = Some(req_approval);
    }

    if let Some(title) = &payload.title {
        changeset.title = Some(title.trim().to_string());
    }
    if let Some(desc) = &payload.description {
        changeset.description = Some(desc.clone());
    }
    if let Some(category) = &payload.category {
        changeset.category = Some(category.clone());
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
    if let Some(limit) = &payload.max_attendees {
        changeset.max_attendees = Some(*limit);
    }
    if let Some(is_online) = payload.is_online {
        changeset.is_online = Some(is_online);
    }
    if let Some(meeting_url) = &payload.meeting_url {
        changeset.meeting_url = Some(
            meeting_url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        );
    }

    let (new_starts, new_ends) = dates;
    changeset.starts_at = Some(new_starts);
    changeset.ends_at = Some(new_ends);
    changeset.updated_at = Some(Utc::now());

    changeset
}

/// Build an `EventChangeset` for an update, after validating creator ownership
/// and input fields. Pure validation — no DB access.
pub(in crate::api) fn prepare_update_changeset(
    headers: &HeaderMap,
    event: &Event,
    profile_id: Uuid,
    payload: &UpdateEventBody,
) -> std::result::Result<EventChangeset, Box<axum::response::Response>> {
    if event.creator_id != profile_id {
        return Err(Box::new(forbidden(
            headers,
            "Only the creator can update this event",
        )));
    }
    validate_event_basic_fields(headers, payload)?;
    let dates = parse_event_dates(headers, event, payload)?;
    Ok(build_update_changeset(payload, dates))
}
