#[path = "view_attendees.rs"]
mod events_view_attendees;
#[path = "view_images.rs"]
mod events_view_images;
#[path = "view_repo.rs"]
mod events_view_repo;

use axum::response::IntoResponse;
use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::api::state::{AttendeeStatus, DataResponse, EventResponse, ProfilePreview};
use crate::db::models::events::Event;
use events_view_repo::load_event_batch_context;

pub(in crate::api) use events_view_attendees::attendee_info;
pub(in crate::api) use events_view_images::resolve_event_images;

const PREVIEW_LIMIT: usize = 5;

fn status_from_str(s: &str) -> AttendeeStatus {
    match s {
        "going" => AttendeeStatus::Going,
        "interested" => AttendeeStatus::Interested,
        "pending" => AttendeeStatus::Pending,
        _ => AttendeeStatus::Invited,
    }
}

fn unknown_preview(id: Uuid) -> ProfilePreview {
    ProfilePreview {
        id: id.to_string(),
        name: "Unknown".to_string(),
        profile_picture: None,
    }
}

fn build_from_context(
    event: &Event,
    profile_id: &Uuid,
    ctx: &events_view_repo::EventBatchContext,
) -> EventResponse {
    let attendee_rows = ctx.attendees.get(&event.id).cloned().unwrap_or_default();

    let attendees_count = attendee_rows
        .iter()
        .filter(|a| a.status == AttendeeStatus::Going)
        .count();

    let attendees_preview = attendee_rows
        .iter()
        .filter(|a| a.status == AttendeeStatus::Going)
        .take(PREVIEW_LIMIT)
        .map(|a| ProfilePreview {
            id: a.profile.id.to_string(),
            name: a.profile.name.clone(),
            profile_picture: a.profile.profile_picture.clone(),
        })
        .collect::<Vec<_>>();

    let is_attending = attendee_rows
        .iter()
        .any(|a| a.profile.id == *profile_id && a.status == AttendeeStatus::Going);

    let is_pending = attendee_rows
        .iter()
        .any(|a| a.profile.id == *profile_id && a.status == AttendeeStatus::Pending);

    let creator = ctx
        .creators
        .get(&event.creator_id)
        .cloned()
        .unwrap_or_else(|| unknown_preview(event.creator_id));

    let event_tags = ctx.tags.get(&event.id).cloned().unwrap_or_default();
    let is_saved = ctx.saved_event_ids.contains(&event.id);

    EventResponse {
        id: event.id.to_string(),
        title: event.title.clone(),
        description: event.description.clone(),
        cover_image: event.cover_image.clone(),
        category: event.category.clone(),
        location: event.location.clone(),
        latitude: event.latitude,
        longitude: event.longitude,
        starts_at: event.starts_at.to_rfc3339(),
        ends_at: event.ends_at.map(|v| v.to_rfc3339()),
        created_at: event.created_at.to_rfc3339(),
        updated_at: event.updated_at.to_rfc3339(),
        creator,
        attendees_count,
        max_attendees: event.max_attendees,
        attendees_preview,
        tags: event_tags,
        is_attending,
        is_saved,
        is_pending,
        requires_approval: event.requires_approval,
        conversation_id: event.conversation_id.clone(),
        recurrence_rule: event.recurrence_rule.clone(),
        visibility: event.visibility.clone(),
        score: None,
    }
}

/// Build raw event responses inside an existing viewer-scoped transaction.
/// Image URLs are not resolved — callers must apply `resolve_event_images`
/// after the transaction closes (avoids holding a DB connection while we
/// call into imgproxy).
pub(in crate::api) async fn build_event_responses_raw(
    conn: &mut AsyncPgConnection,
    event_models: &[Event],
    profile_id: &Uuid,
) -> std::result::Result<Vec<EventResponse>, crate::error::AppError> {
    let batch_ctx = load_event_batch_context(conn, event_models, *profile_id).await?;
    Ok(event_models
        .iter()
        .map(|event| build_from_context(event, profile_id, &batch_ctx))
        .collect())
}

pub(in crate::api) async fn build_event_response_raw(
    conn: &mut AsyncPgConnection,
    event: &Event,
    profile_id: &Uuid,
) -> std::result::Result<EventResponse, crate::error::AppError> {
    let mut responses =
        build_event_responses_raw(conn, std::slice::from_ref(event), profile_id).await?;
    responses.pop().ok_or_else(|| {
        crate::error::AppError::Message("Failed to build event response".to_string())
    })
}

pub(in crate::api) fn created_event_response(data: EventResponse) -> axum::response::Response {
    (
        axum::http::StatusCode::CREATED,
        axum::Json(DataResponse { data }),
    )
        .into_response()
}
