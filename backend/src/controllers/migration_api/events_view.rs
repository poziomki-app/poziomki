use axum::{response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::prelude::*;

use crate::controllers::migration_api::state::{
    to_profile_preview, AttendeeFullInfo, AttendeeStatus, DataResponse, EventRecord, EventResponse,
    EventTagResponse, MigrationState, ProfilePreview, ProfileRecord,
};

const PREVIEW_LIMIT: usize = 5;

fn creator_preview(state: &MigrationState, creator_id: &str) -> ProfilePreview {
    state.profiles.get(creator_id).map_or_else(
        || ProfilePreview {
            id: creator_id.to_string(),
            name: "Unknown".to_string(),
            profile_picture: None,
        },
        to_profile_preview,
    )
}

fn attendee_rows<'a>(
    state: &'a MigrationState,
    event_id: &str,
) -> Vec<(&'a ProfileRecord, AttendeeStatus)> {
    let mut rows = state
        .event_attendees
        .iter()
        .filter(|((stored_event_id, _), _)| stored_event_id == event_id)
        .filter_map(|((_, profile_id), status)| {
            state
                .profiles
                .get(profile_id)
                .map(|profile| (profile, *status))
        })
        .collect::<Vec<_>>();

    rows.sort_by(|(left, _), (right, _)| left.name.cmp(&right.name));
    rows
}

fn event_tags(state: &MigrationState, event: &EventRecord) -> Vec<EventTagResponse> {
    event
        .tag_ids
        .iter()
        .filter_map(|tag_id| state.tags.get(tag_id))
        .map(|tag| EventTagResponse {
            id: tag.id.clone(),
            name: tag.name.clone(),
            scope: tag.scope,
        })
        .collect::<Vec<_>>()
}

pub(in crate::controllers::migration_api) fn event_response(
    state: &MigrationState,
    event: &EventRecord,
    profile_id: &str,
) -> EventResponse {
    let attendees = attendee_rows(state, &event.id);
    let attendees_count = attendees
        .iter()
        .filter(|(_, status)| *status == AttendeeStatus::Going)
        .count();

    let attendees_preview = attendees
        .iter()
        .filter(|(_, status)| *status == AttendeeStatus::Going)
        .take(PREVIEW_LIMIT)
        .map(|(profile, _)| to_profile_preview(profile))
        .collect::<Vec<_>>();

    let is_attending = attendees
        .iter()
        .any(|(profile, status)| profile.id == profile_id && *status == AttendeeStatus::Going);

    EventResponse {
        id: event.id.clone(),
        title: event.title.clone(),
        description: event.description.clone(),
        cover_image: event.cover_image.clone(),
        location: event.location.clone(),
        starts_at: event.starts_at.to_rfc3339(),
        ends_at: event.ends_at.map(|value| value.to_rfc3339()),
        created_at: event.created_at.to_rfc3339(),
        updated_at: event.updated_at.to_rfc3339(),
        creator: creator_preview(state, &event.creator_id),
        attendees_count,
        attendees_preview,
        tags: event_tags(state, event),
        is_attending,
        conversation_id: event.conversation_id.clone(),
    }
}

pub(in crate::controllers::migration_api) fn sorted_event_ids(
    state: &MigrationState,
    include_past: bool,
) -> Vec<String> {
    let now = Utc::now();
    let mut ids = state
        .events
        .values()
        .filter(|event| include_past || event.starts_at >= now)
        .map(|event| event.id.clone())
        .collect::<Vec<_>>();

    ids.sort_by(|left, right| {
        state
            .events
            .get(left)
            .zip(state.events.get(right))
            .map_or(std::cmp::Ordering::Equal, |(l, r)| {
                l.starts_at.cmp(&r.starts_at)
            })
    });

    ids
}

pub(in crate::controllers::migration_api) fn attendee_info(
    state: &MigrationState,
    event_id: &str,
) -> Vec<AttendeeFullInfo> {
    attendee_rows(state, event_id)
        .into_iter()
        .map(|(profile, status)| AttendeeFullInfo {
            id: profile.id.clone(),
            user_id: profile.user_id.clone(),
            name: profile.name.clone(),
            profile_picture: profile.profile_picture.clone(),
            status,
        })
        .collect::<Vec<_>>()
}

pub(in crate::controllers::migration_api) fn created_event_response(
    data: EventResponse,
) -> Response {
    (axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response()
}
