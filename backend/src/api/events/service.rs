use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::{error_response, state::CreateEventBody, ErrorSpec};
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::schema::{events, profiles};

pub(in crate::api) type EventDates = (String, DateTime<Utc>, Option<DateTime<Utc>>);
pub(in crate::api) type HandlerError = Box<axum::response::Response>;

pub(in crate::api) fn validation_error(
    headers: &HeaderMap,
    message: &str,
) -> axum::response::Response {
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

pub(in crate::api) fn not_found_event(
    headers: &HeaderMap,
    event_id: &str,
) -> axum::response::Response {
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

pub(in crate::api) fn profile_not_found(headers: &HeaderMap) -> axum::response::Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: "Profile not found. Create a profile first.".to_string(),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

pub(in crate::api) fn forbidden(headers: &HeaderMap, message: &str) -> axum::response::Response {
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

/// Load the caller's profile inside an existing viewer-scoped transaction.
/// Returns `None` when the caller has no profile; callers translate that into
/// a 404 response.
pub(in crate::api) async fn load_profile_for_user(
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> std::result::Result<Option<Profile>, crate::error::AppError> {
    let profile = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(conn)
        .await
        .optional()?;
    Ok(profile)
}

/// Load an event by id inside an existing viewer-scoped transaction.
pub(in crate::api) async fn load_event_by_id(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
) -> std::result::Result<Option<Event>, crate::error::AppError> {
    let event = events::table
        .find(event_id)
        .first::<Event>(conn)
        .await
        .optional()?;
    Ok(event)
}

pub(in crate::api) fn parse_timestamp(
    value: &str,
) -> std::result::Result<DateTime<Utc>, &'static str> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| "Invalid date-time format")
}

pub(in crate::api) fn validate_event_description(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 2_000) {
        Err("Description must be at most 2000 characters")
    } else {
        Ok(())
    }
}

pub(in crate::api) fn validate_event_location(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 500) {
        Err("Location must be at most 500 characters")
    } else {
        Ok(())
    }
}

pub(in crate::api) fn validate_event_category(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 64) {
        Err("Category must be at most 64 characters")
    } else {
        Ok(())
    }
}

pub(in crate::api) const MAX_ATTENDEES_UPPER_BOUND: i32 = 10_000;

pub(in crate::api) const fn validate_max_attendees(
    value: Option<i32>,
) -> std::result::Result<(), &'static str> {
    match value {
        Some(limit) if limit <= 0 => Err("Attendee limit must be greater than 0"),
        Some(limit) if limit > MAX_ATTENDEES_UPPER_BOUND => {
            Err("Attendee limit must be at most 10000")
        }
        _ => Ok(()),
    }
}

fn validate_event_title(value: &str) -> std::result::Result<String, &'static str> {
    let normalized = value.trim();
    let length = normalized.chars().count();
    if length == 0 {
        Err("Title is required")
    } else if length > 200 {
        Err("Title must be at most 200 characters")
    } else {
        Ok(normalized.to_string())
    }
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

fn ensure_valid_event_range(
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
) -> std::result::Result<(), ErrorSpec> {
    if ends_at.is_some_and(|end| end <= starts_at) {
        Err(ErrorSpec {
            error: "Event end time must be after start time".to_string(),
            code: "INVALID_DATE_RANGE",
            details: None,
        })
    } else {
        Ok(())
    }
}

pub(in crate::api) fn parse_create_dates(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<EventDates, HandlerError> {
    let title = parse_valid_title(headers, &payload.title)?;

    validate_event_description(payload.description.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    validate_event_category(payload.category.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    validate_event_location(payload.location.as_ref())
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;

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
