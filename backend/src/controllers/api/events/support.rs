use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::controllers::api::{error_response, state::CreateEventBody, ErrorSpec};
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::schema::{events, profiles};

pub(in crate::controllers::api) type EventDates = (String, DateTime<Utc>, Option<DateTime<Utc>>);
pub(in crate::controllers::api) type HandlerError = Box<axum::response::Response>;

pub(in crate::controllers::api) fn validation_error(
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

pub(in crate::controllers::api) fn not_found_event(
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

pub(in crate::controllers::api) fn forbidden(
    headers: &HeaderMap,
    message: &str,
) -> axum::response::Response {
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

pub(in crate::controllers::api) async fn require_auth_profile(
    headers: &HeaderMap,
) -> std::result::Result<(Profile, Uuid), HandlerError> {
    let (_session, user) = crate::controllers::api::state::require_auth_db(headers).await?;

    let mut conn = crate::db::conn().await.map_err(|_| {
        Box::new(error_response(
            axum::http::StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: "Profile not found. Create a profile first.".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ))
    })?;

    let profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|_| {
            Box::new(error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: "Profile not found. Create a profile first.".to_string(),
                    code: "NOT_FOUND",
                    details: None,
                },
            ))
        })?
        .ok_or_else(|| {
            Box::new(error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: "Profile not found. Create a profile first.".to_string(),
                    code: "NOT_FOUND",
                    details: None,
                },
            ))
        })?;
    Ok((profile, user.pid))
}

pub(in crate::controllers::api) async fn load_event(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Event, Uuid), HandlerError> {
    let event_uuid = Uuid::parse_str(id).map_err(|_| {
        Box::new(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Invalid event ID".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ))
    })?;

    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(not_found_event(headers, id)))?;

    let event = events::table
        .find(event_uuid)
        .first::<Event>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found_event(headers, id)))?
        .ok_or_else(|| Box::new(not_found_event(headers, id)))?;

    Ok((event, event_uuid))
}

pub(in crate::controllers::api) fn parse_timestamp(
    value: &str,
) -> std::result::Result<DateTime<Utc>, &'static str> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| "Invalid date-time format")
}

pub(in crate::controllers::api) fn validate_event_description(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 2_000) {
        Err("Description must be at most 2000 characters")
    } else {
        Ok(())
    }
}

pub(in crate::controllers::api) fn validate_event_location(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 500) {
        Err("Location must be at most 500 characters")
    } else {
        Ok(())
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

pub(in crate::controllers::api) fn parse_create_dates(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<EventDates, HandlerError> {
    let title = parse_valid_title(headers, &payload.title)?;

    validate_event_description(payload.description.as_ref())
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
