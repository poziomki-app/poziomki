use std::collections::{HashMap, HashSet};

use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::controllers::migration_api::state::{
    AttendeeFullInfo, AttendeeStatus, DataResponse, EventResponse, EventTagResponse,
    ProfilePreview, TagScope,
};
use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::models::tags::Tag;
use crate::db::models::users::User;
use crate::db::schema::{event_attendees, event_tags, profiles, tags, users};

const PREVIEW_LIMIT: usize = 5;

fn status_from_str(s: &str) -> AttendeeStatus {
    match s {
        "going" => AttendeeStatus::Going,
        "interested" => AttendeeStatus::Interested,
        _ => AttendeeStatus::Invited,
    }
}

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

fn unknown_preview(id: Uuid) -> ProfilePreview {
    ProfilePreview {
        id: id.to_string(),
        name: "Unknown".to_string(),
        profile_picture: None,
    }
}

#[derive(Clone)]
struct AttendeeRow {
    profile: Profile,
    status: AttendeeStatus,
}

struct EventBatchContext {
    creators: HashMap<Uuid, ProfilePreview>,
    attendees: HashMap<Uuid, Vec<AttendeeRow>>,
    tags: HashMap<Uuid, Vec<EventTagResponse>>,
}

async fn load_profiles_by_ids(
    ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Profile>, crate::error::AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let models = profiles::table
        .filter(profiles::id.eq_any(ids))
        .load::<Profile>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(models.into_iter().map(|m| (m.id, m)).collect())
}

async fn load_attendee_rows(
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeRow>, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let links = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .load::<EventAttendee>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let profile_ids: HashSet<Uuid> = links.iter().map(|a| a.profile_id).collect();
    if profile_ids.is_empty() {
        return Ok(vec![]);
    }

    let profiles_map = load_profiles_by_ids(&profile_ids.into_iter().collect::<Vec<_>>()).await?;

    let mut rows: Vec<AttendeeRow> = links
        .iter()
        .filter_map(|link| {
            profiles_map
                .get(&link.profile_id)
                .cloned()
                .map(|profile| AttendeeRow {
                    profile,
                    status: status_from_str(&link.status),
                })
        })
        .collect();

    rows.sort_by(|a, b| a.profile.name.cmp(&b.profile.name));
    Ok(rows)
}

async fn load_event_batch_context(
    event_models: &[Event],
) -> std::result::Result<EventBatchContext, crate::error::AppError> {
    if event_models.is_empty() {
        return Ok(EventBatchContext {
            creators: HashMap::new(),
            attendees: HashMap::new(),
            tags: HashMap::new(),
        });
    }

    let event_ids: Vec<Uuid> = event_models.iter().map(|e| e.id).collect();

    let creator_ids: HashSet<Uuid> = event_models.iter().map(|e| e.creator_id).collect();
    let creator_models = load_profiles_by_ids(&creator_ids.into_iter().collect::<Vec<_>>()).await?;
    let creators: HashMap<Uuid, ProfilePreview> = creator_models
        .into_values()
        .map(|profile| {
            (
                profile.id,
                ProfilePreview {
                    id: profile.id.to_string(),
                    name: profile.name,
                    profile_picture: profile.profile_picture,
                },
            )
        })
        .collect();

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let attendee_links = event_attendees::table
        .filter(event_attendees::event_id.eq_any(&event_ids))
        .load::<EventAttendee>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let attendee_profile_ids: HashSet<Uuid> = attendee_links.iter().map(|a| a.profile_id).collect();
    let attendee_profiles =
        load_profiles_by_ids(&attendee_profile_ids.into_iter().collect::<Vec<_>>()).await?;

    let mut attendees: HashMap<Uuid, Vec<AttendeeRow>> = HashMap::new();
    for link in &attendee_links {
        if let Some(profile) = attendee_profiles.get(&link.profile_id) {
            attendees
                .entry(link.event_id)
                .or_default()
                .push(AttendeeRow {
                    profile: profile.clone(),
                    status: status_from_str(&link.status),
                });
        }
    }
    for rows in attendees.values_mut() {
        rows.sort_by(|a, b| a.profile.name.cmp(&b.profile.name));
    }

    let tag_links = event_tags::table
        .filter(event_tags::event_id.eq_any(&event_ids))
        .load::<EventTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_ids: HashSet<Uuid> = tag_links.iter().map(|l| l.tag_id).collect();
    let tag_models: HashMap<Uuid, Tag> = if tag_ids.is_empty() {
        HashMap::new()
    } else {
        tags::table
            .filter(tags::id.eq_any(&tag_ids.into_iter().collect::<Vec<_>>()))
            .load::<Tag>(&mut conn)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?
            .into_iter()
            .map(|tag| (tag.id, tag))
            .collect()
    };

    let mut tags_by_event: HashMap<Uuid, Vec<EventTagResponse>> = HashMap::new();
    for link in &tag_links {
        if let Some(tag) = tag_models.get(&link.tag_id) {
            tags_by_event
                .entry(link.event_id)
                .or_default()
                .push(EventTagResponse {
                    id: tag.id.to_string(),
                    name: tag.name.clone(),
                    scope: scope_from_str(&tag.scope),
                });
        }
    }

    Ok(EventBatchContext {
        creators,
        attendees,
        tags: tags_by_event,
    })
}

fn build_from_context(event: &Event, profile_id: &Uuid, ctx: &EventBatchContext) -> EventResponse {
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

    let creator = ctx
        .creators
        .get(&event.creator_id)
        .cloned()
        .unwrap_or_else(|| unknown_preview(event.creator_id));

    let event_tags = ctx.tags.get(&event.id).cloned().unwrap_or_default();

    EventResponse {
        id: event.id.to_string(),
        title: event.title.clone(),
        description: event.description.clone(),
        cover_image: event.cover_image.clone(),
        location: event.location.clone(),
        latitude: event.latitude,
        longitude: event.longitude,
        starts_at: event.starts_at.to_rfc3339(),
        ends_at: event.ends_at.map(|v| v.to_rfc3339()),
        created_at: event.created_at.to_rfc3339(),
        updated_at: event.updated_at.to_rfc3339(),
        creator,
        attendees_count,
        attendees_preview,
        tags: event_tags,
        is_attending,
        conversation_id: event.conversation_id.clone(),
        score: None,
    }
}

/// Resolve a set of raw image filenames to signed URLs in parallel, returning a lookup map.
async fn resolve_image_map(raw_values: HashSet<String>) -> HashMap<String, String> {
    let resolve = crate::controllers::migration_api::resolve_image_url;
    let futs: Vec<_> = raw_values
        .into_iter()
        .map(|raw| async move {
            let resolved = resolve(&raw).await;
            (raw, resolved)
        })
        .collect();
    futures::future::join_all(futs).await.into_iter().collect()
}

fn collect_image_filenames(responses: &[EventResponse]) -> HashSet<String> {
    let mut filenames = HashSet::new();
    for r in responses {
        if let Some(v) = &r.cover_image {
            filenames.insert(v.clone());
        }
        if let Some(v) = &r.creator.profile_picture {
            filenames.insert(v.clone());
        }
        for a in &r.attendees_preview {
            if let Some(v) = &a.profile_picture {
                filenames.insert(v.clone());
            }
        }
    }
    filenames
}

fn replace_resolved_image(value: &mut Option<String>, url_map: &HashMap<String, String>) {
    if let Some(resolved) = value
        .as_ref()
        .and_then(|raw| url_map.get(raw.as_str()))
        .cloned()
    {
        *value = Some(resolved);
    }
}

/// Resolve all image URLs (cover, creator, attendee previews) in event responses.
async fn resolve_event_images(responses: &mut [EventResponse]) {
    let filenames = collect_image_filenames(responses);
    if filenames.is_empty() {
        return;
    }
    let url_map = resolve_image_map(filenames).await;

    for response in responses.iter_mut() {
        replace_resolved_image(&mut response.cover_image, &url_map);
        replace_resolved_image(&mut response.creator.profile_picture, &url_map);
        for preview in &mut response.attendees_preview {
            replace_resolved_image(&mut preview.profile_picture, &url_map);
        }
    }
}

pub(in crate::controllers::migration_api) async fn build_event_responses(
    event_models: &[Event],
    profile_id: &Uuid,
) -> std::result::Result<Vec<EventResponse>, crate::error::AppError> {
    let batch_ctx = load_event_batch_context(event_models).await?;
    let mut responses: Vec<EventResponse> = event_models
        .iter()
        .map(|event| build_from_context(event, profile_id, &batch_ctx))
        .collect();
    resolve_event_images(&mut responses).await;
    Ok(responses)
}

pub(in crate::controllers::migration_api) async fn build_event_response(
    event: &Event,
    profile_id: &Uuid,
) -> std::result::Result<EventResponse, crate::error::AppError> {
    let responses = build_event_responses(std::slice::from_ref(event), profile_id).await?;
    responses.into_iter().next().ok_or_else(|| {
        crate::error::AppError::Message("Failed to build event response".to_string())
    })
}

pub(in crate::controllers::migration_api) async fn attendee_info(
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeFullInfo>, crate::error::AppError> {
    let rows = load_attendee_rows(event_id).await?;

    let user_ids: Vec<i32> = rows.iter().map(|r| r.profile.user_id).collect();
    let user_models = if user_ids.is_empty() {
        vec![]
    } else {
        let mut conn = crate::db::conn()
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
        users::table
            .filter(users::id.eq_any(&user_ids))
            .load::<User>(&mut conn)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?
    };

    let filenames: HashSet<String> = rows
        .iter()
        .filter_map(|r| r.profile.profile_picture.clone())
        .collect();
    let url_map = resolve_image_map(filenames).await;

    Ok(rows
        .into_iter()
        .map(|row| {
            let user_pid = user_models
                .iter()
                .find(|u| u.id == row.profile.user_id)
                .map_or(uuid::Uuid::nil(), |u| u.pid);
            let profile_picture = row
                .profile
                .profile_picture
                .as_ref()
                .and_then(|raw| url_map.get(raw.as_str()))
                .cloned();
            AttendeeFullInfo {
                id: row.profile.id.to_string(),
                user_id: user_pid.to_string(),
                name: row.profile.name.clone(),
                profile_picture,
                status: row.status,
            }
        })
        .collect())
}

pub(in crate::controllers::migration_api) fn created_event_response(
    data: EventResponse,
) -> axum::response::Response {
    (
        axum::http::StatusCode::CREATED,
        axum::Json(DataResponse { data }),
    )
        .into_response()
}
