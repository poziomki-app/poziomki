use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use loco_rs::prelude::*;

use crate::controllers::migration_api::{
    error_response,
    state::{
        ensure_valid_event_range, parse_timestamp, require_auth, require_profile,
        resolve_event_tag_ids, validate_event_description, validate_event_location,
        validate_event_title, CreateEventBody, EventRecord, MigrationState, ProfileRecord,
        UpdateEventBody,
    },
    ErrorSpec,
};

pub(in crate::controllers::migration_api) type EventDates =
    (String, DateTime<Utc>, Option<DateTime<Utc>>);
pub(in crate::controllers::migration_api) type HandlerError = Box<Response>;

pub(in crate::controllers::migration_api) struct EventUpdateInput {
    pub(in crate::controllers::migration_api) starts_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) ends_at: Option<DateTime<Utc>>,
    pub(in crate::controllers::migration_api) description: Option<String>,
    pub(in crate::controllers::migration_api) location: Option<String>,
}

pub(in crate::controllers::migration_api) fn validation_error(
    headers: &HeaderMap,
    message: &str,
) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

pub(in crate::controllers::migration_api) fn not_found_event(
    headers: &HeaderMap,
    event_id: &str,
) -> Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: format!("Event '{event_id}' not found"),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

pub(in crate::controllers::migration_api) fn forbidden(
    headers: &HeaderMap,
    message: &str,
) -> Response {
    error_response(
        axum::http::StatusCode::FORBIDDEN,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "FORBIDDEN",
            details: None,
        },
    )
}

pub(in crate::controllers::migration_api) fn internal_error(
    headers: &HeaderMap,
    message: &str,
) -> Response {
    error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    )
}

pub(in crate::controllers::migration_api) fn auth_profile(
    headers: &HeaderMap,
    state: &mut MigrationState,
) -> std::result::Result<ProfileRecord, HandlerError> {
    let (_session, user) = require_auth(headers, state)?;
    require_profile(headers, state, &user.id)
}

fn parse_valid_title(
    headers: &HeaderMap,
    value: &str,
) -> std::result::Result<String, HandlerError> {
    validate_event_title(value).map_err(|msg| Box::new(validation_error(headers, msg)))
}

fn parse_required_timestamp(
    headers: &HeaderMap,
    value: &str,
) -> std::result::Result<DateTime<Utc>, HandlerError> {
    parse_timestamp(value).map_err(|msg| Box::new(validation_error(headers, msg)))
}

fn parse_optional_timestamp(
    headers: &HeaderMap,
    value: Option<&str>,
) -> std::result::Result<Option<DateTime<Utc>>, HandlerError> {
    value
        .map(|raw| parse_required_timestamp(headers, raw))
        .transpose()
}

fn validate_create_text_fields(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<(), HandlerError> {
    validate_event_description(payload.description.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    validate_event_location(payload.location.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    Ok(())
}

pub(in crate::controllers::migration_api) fn parse_create_dates(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<EventDates, HandlerError> {
    let title = parse_valid_title(headers, &payload.title)?;
    validate_create_text_fields(headers, payload)?;
    let starts_at = parse_required_timestamp(headers, &payload.starts_at)?;
    let ends_at = parse_optional_timestamp(headers, payload.ends_at.as_deref())?;

    ensure_valid_event_range(starts_at, ends_at).map_err(|spec| {
        Box::new(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            spec,
        ))
    })?;

    Ok((title, starts_at, ends_at))
}

fn ensure_event_owner(
    headers: &HeaderMap,
    event: &EventRecord,
    profile_id: &str,
) -> Option<Response> {
    (event.creator_id != profile_id)
        .then(|| forbidden(headers, "Only the creator can update this event"))
}

pub(in crate::controllers::migration_api) fn ensure_event_exists_for_update(
    headers: &HeaderMap,
    state: &MigrationState,
    id: &str,
) -> std::result::Result<EventRecord, HandlerError> {
    state
        .events
        .get(id)
        .cloned()
        .ok_or_else(|| Box::new(not_found_event(headers, id)))
}

pub(in crate::controllers::migration_api) fn ensure_update_permission(
    headers: &HeaderMap,
    event: &EventRecord,
    profile_id: &str,
) -> std::result::Result<(), HandlerError> {
    ensure_event_owner(headers, event, profile_id)
        .map_or(Ok(()), |response| Err(Box::new(response)))
}

pub(in crate::controllers::migration_api) fn update_dates(
    headers: &HeaderMap,
    existing: &EventRecord,
    payload: &UpdateEventBody,
) -> std::result::Result<(DateTime<Utc>, Option<DateTime<Utc>>), HandlerError> {
    let starts_at = payload
        .starts_at
        .as_deref()
        .map(|value| parse_required_timestamp(headers, value))
        .transpose()?
        .unwrap_or(existing.starts_at);

    let ends_at = match payload.ends_at.as_ref() {
        Some(value) => parse_optional_timestamp(headers, value.as_ref().map(String::as_str))?,
        None => existing.ends_at,
    };

    ensure_valid_event_range(starts_at, ends_at).map_err(|spec| {
        Box::new(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            spec,
        ))
    })?;

    Ok((starts_at, ends_at))
}

fn merged_description(
    headers: &HeaderMap,
    payload: &UpdateEventBody,
    existing: &EventRecord,
) -> std::result::Result<Option<String>, HandlerError> {
    let description = payload
        .description
        .clone()
        .unwrap_or_else(|| existing.description.clone());
    validate_event_description(description.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    Ok(description)
}

fn merged_location(
    headers: &HeaderMap,
    payload: &UpdateEventBody,
    existing: &EventRecord,
) -> std::result::Result<Option<String>, HandlerError> {
    let location = payload
        .location
        .clone()
        .unwrap_or_else(|| existing.location.clone());
    validate_event_location(location.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    Ok(location)
}

pub(in crate::controllers::migration_api) fn build_update_input(
    headers: &HeaderMap,
    existing: &EventRecord,
    payload: &UpdateEventBody,
) -> std::result::Result<EventUpdateInput, HandlerError> {
    let (starts_at, ends_at) = update_dates(headers, existing, payload)?;
    let description = merged_description(headers, payload, existing)?;
    let location = merged_location(headers, payload, existing)?;

    Ok(EventUpdateInput {
        starts_at,
        ends_at,
        description,
        location,
    })
}

pub(in crate::controllers::migration_api) fn apply_event_update(
    state: &mut MigrationState,
    existing: EventRecord,
    payload: UpdateEventBody,
    input: EventUpdateInput,
) -> EventRecord {
    let mut updated = existing;
    updated.title = payload
        .title
        .map_or_else(|| updated.title.clone(), |value| value.trim().to_string());
    updated.description = input.description;
    updated.cover_image = payload
        .cover_image
        .unwrap_or_else(|| updated.cover_image.clone());
    updated.location = input.location;
    updated.starts_at = input.starts_at;
    updated.ends_at = input.ends_at;
    updated.updated_at = Utc::now();
    if payload.tags.is_some() || payload.tag_ids.is_some() {
        updated.tag_ids = resolve_event_tag_ids(state, payload.tags, payload.tag_ids);
    }
    updated
}
