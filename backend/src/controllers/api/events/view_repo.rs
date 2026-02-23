use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::status_from_str;
use crate::controllers::api::state::{EventTagResponse, ProfilePreview, TagScope};
use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::models::tags::Tag;
use crate::db::schema::{event_attendees, event_tags, profiles, tags};

#[derive(Clone)]
pub(super) struct AttendeeRow {
    pub(super) profile: Profile,
    pub(super) status: crate::controllers::api::state::AttendeeStatus,
}

pub(super) struct EventBatchContext {
    pub(super) creators: HashMap<Uuid, ProfilePreview>,
    pub(super) attendees: HashMap<Uuid, Vec<AttendeeRow>>,
    pub(super) tags: HashMap<Uuid, Vec<EventTagResponse>>,
}

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

async fn load_profiles_by_ids(
    ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Profile>, crate::error::AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = crate::db::conn().await?;
    let models = profiles::table
        .filter(profiles::id.eq_any(ids))
        .load::<Profile>(&mut conn)
        .await?;
    Ok(models.into_iter().map(|model| (model.id, model)).collect())
}

pub(super) async fn load_attendee_rows(
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeRow>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let links = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .load::<EventAttendee>(&mut conn)
        .await?;

    let profile_ids: HashSet<Uuid> = links.iter().map(|attendee| attendee.profile_id).collect();
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

async fn load_creator_previews(
    events_list: &[Event],
) -> std::result::Result<HashMap<Uuid, ProfilePreview>, crate::error::AppError> {
    let creator_ids: HashSet<Uuid> = events_list.iter().map(|event| event.creator_id).collect();
    let creator_models = load_profiles_by_ids(&creator_ids.into_iter().collect::<Vec<_>>()).await?;

    Ok(creator_models
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
        .collect())
}

async fn load_event_attendee_map(
    event_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Vec<AttendeeRow>>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let attendee_links = event_attendees::table
        .filter(event_attendees::event_id.eq_any(event_ids))
        .load::<EventAttendee>(&mut conn)
        .await?;

    let attendee_profile_ids: HashSet<Uuid> = attendee_links
        .iter()
        .map(|attendee| attendee.profile_id)
        .collect();
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

    Ok(attendees)
}

async fn load_event_tag_map(
    event_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Vec<EventTagResponse>>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let tag_links = event_tags::table
        .filter(event_tags::event_id.eq_any(event_ids))
        .load::<EventTag>(&mut conn)
        .await?;

    let tag_ids: HashSet<Uuid> = tag_links.iter().map(|link| link.tag_id).collect();
    let tag_models: HashMap<Uuid, Tag> = if tag_ids.is_empty() {
        HashMap::new()
    } else {
        tags::table
            .filter(tags::id.eq_any(&tag_ids.into_iter().collect::<Vec<_>>()))
            .load::<Tag>(&mut conn)
            .await?
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

    Ok(tags_by_event)
}

pub(super) async fn load_event_batch_context(
    event_models: &[Event],
) -> std::result::Result<EventBatchContext, crate::error::AppError> {
    if event_models.is_empty() {
        return Ok(EventBatchContext {
            creators: HashMap::new(),
            attendees: HashMap::new(),
            tags: HashMap::new(),
        });
    }

    let event_ids: Vec<Uuid> = event_models.iter().map(|event| event.id).collect();
    let creators = load_creator_previews(event_models).await?;
    let attendees = load_event_attendee_map(&event_ids).await?;
    let tags = load_event_tag_map(&event_ids).await?;

    Ok(EventBatchContext {
        creators,
        attendees,
        tags,
    })
}
