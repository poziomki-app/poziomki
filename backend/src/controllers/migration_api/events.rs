#[path = "events_support.rs"]
mod events_support;
#[path = "events_view.rs"]
mod events_view;

use axum::{
    extract::{Path, Query},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::prelude::*;
use std::cmp::Reverse;
use uuid::Uuid;

use super::state::{
    bounded_limit, lock_state, resolve_event_tag_ids, AttendEventBody, AttendeeStatus,
    CreateEventBody, DataResponse, EventRecord, EventsQuery, SuccessResponse, UpdateEventBody,
};
use events_support::{
    apply_event_update, auth_profile, build_update_input, ensure_event_exists_for_update,
    ensure_update_permission, forbidden, internal_error, not_found_event, parse_create_dates,
    HandlerError,
};
use events_view::{attendee_info, created_event_response, event_response, sorted_event_ids};

pub(super) async fn events_list(
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Response> {
    let mut state = lock_state();
    let profile = match auth_profile(&headers, &mut state) {
        Ok(profile) => profile,
        Err(response) => return Ok(*response),
    };

    let limit = bounded_limit(query.limit);
    let data = sorted_event_ids(&state, false)
        .into_iter()
        .take(limit)
        .filter_map(|id| state.events.get(&id))
        .map(|event| event_response(&state, event, &profile.id))
        .collect::<Vec<_>>();
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn events_mine(headers: HeaderMap) -> Result<Response> {
    let mut state = lock_state();
    let profile = match auth_profile(&headers, &mut state) {
        Ok(profile) => profile,
        Err(response) => return Ok(*response),
    };

    let mut events = state
        .events
        .values()
        .filter(|event| event.creator_id == profile.id)
        .collect::<Vec<_>>();
    events.sort_by_key(|event| Reverse(event.starts_at));

    let data = events
        .into_iter()
        .map(|event| event_response(&state, event, &profile.id))
        .collect::<Vec<_>>();
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn event_get(headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    let mut state = lock_state();
    let profile = match auth_profile(&headers, &mut state) {
        Ok(profile) => profile,
        Err(response) => return Ok(*response),
    };

    let Some(event) = state.events.get(&id) else {
        return Ok(not_found_event(&headers, &id));
    };
    let data = event_response(&state, event, &profile.id);
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn event_attendees(
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let mut state = lock_state();
    let _profile = match auth_profile(&headers, &mut state) {
        Ok(profile) => profile,
        Err(response) => return Ok(*response),
    };

    if !state.events.contains_key(&id) {
        return Ok(not_found_event(&headers, &id));
    }

    let data = attendee_info(&state, &id);
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn event_create(
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let profile = auth_profile(&headers, &mut state)?;
        let (title, starts_at, ends_at) = parse_create_dates(&headers, &payload)?;

        let now = Utc::now();
        let event = EventRecord {
            id: Uuid::new_v4().to_string(),
            title,
            description: payload.description,
            cover_image: payload.cover_image,
            location: payload.location,
            starts_at,
            ends_at,
            creator_id: profile.id.clone(),
            conversation_id: None,
            tag_ids: resolve_event_tag_ids(&mut state, payload.tags, payload.tag_ids),
            created_at: now,
            updated_at: now,
        };

        let event_id = event.id.clone();
        state.events.insert(event_id.clone(), event);
        state.event_attendees.insert(
            (event_id.clone(), profile.id.clone()),
            AttendeeStatus::Going,
        );

        let saved_event = state
            .events
            .get(&event_id)
            .ok_or_else(|| Box::new(internal_error(&headers, "Failed to persist created event")))?;
        let data = event_response(&state, saved_event, &profile.id);
        Ok(created_event_response(data))
    })()
    .unwrap_or_else(|response| *response);
    drop(state);

    Ok(response)
}

pub(super) async fn event_update(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let profile = auth_profile(&headers, &mut state)?;
        let existing = ensure_event_exists_for_update(&headers, &state, &id)?;
        ensure_update_permission(&headers, &existing, &profile.id)?;

        let input = build_update_input(&headers, &existing, &payload)?;
        let updated = apply_event_update(&mut state, existing, payload, input);

        state.events.insert(id.clone(), updated);
        let event = state
            .events
            .get(&id)
            .ok_or_else(|| Box::new(internal_error(&headers, "Failed to persist event update")))?;
        let data = event_response(&state, event, &profile.id);
        Ok(Json(DataResponse { data }).into_response())
    })()
    .unwrap_or_else(|response| *response);
    drop(state);

    Ok(response)
}

pub(super) async fn event_delete(headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let profile = auth_profile(&headers, &mut state)?;
        let event = state
            .events
            .get(&id)
            .cloned()
            .ok_or_else(|| Box::new(not_found_event(&headers, &id)))?;

        if event.creator_id != profile.id {
            return Ok(forbidden(
                &headers,
                "Only the creator can delete this event",
            ));
        }

        state.events.remove(&id);
        state
            .event_attendees
            .retain(|(event_id, _), _| event_id != &id);

        Ok(Json(SuccessResponse { success: true }).into_response())
    })()
    .unwrap_or_else(|response| *response);
    drop(state);

    Ok(response)
}

pub(super) async fn event_attend(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<AttendEventBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let profile = auth_profile(&headers, &mut state)?;

        if !state.events.contains_key(&id) {
            return Ok(not_found_event(&headers, &id));
        }

        state.event_attendees.insert(
            (id.clone(), profile.id.clone()),
            payload.status.unwrap_or(AttendeeStatus::Going),
        );

        let event = state
            .events
            .get(&id)
            .ok_or_else(|| Box::new(internal_error(&headers, "Failed to persist attendance")))?;
        let data = event_response(&state, event, &profile.id);
        Ok(Json(DataResponse { data }).into_response())
    })()
    .unwrap_or_else(|response| *response);
    drop(state);

    Ok(response)
}

pub(super) async fn event_leave(headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let profile = auth_profile(&headers, &mut state)?;

        let event = state
            .events
            .get(&id)
            .cloned()
            .ok_or_else(|| Box::new(not_found_event(&headers, &id)))?;
        if event.creator_id == profile.id {
            return Ok(forbidden(&headers, "Event creator cannot leave the event"));
        }

        state
            .event_attendees
            .remove(&(id.clone(), profile.id.clone()));

        let saved_event = state.events.get(&id).ok_or_else(|| {
            Box::new(internal_error(&headers, "Failed to load event after leave"))
        })?;
        let data = event_response(&state, saved_event, &profile.id);
        Ok(Json(DataResponse { data }).into_response())
    })()
    .unwrap_or_else(|response| *response);
    drop(state);

    Ok(response)
}
