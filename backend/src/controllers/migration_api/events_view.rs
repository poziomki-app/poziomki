use axum::response::IntoResponse;
use loco_rs::prelude::*;
use sea_orm::QueryFilter;
use uuid::Uuid;

use crate::controllers::migration_api::state::{
    AttendeeFullInfo, AttendeeStatus, DataResponse, EventResponse, EventTagResponse, ProfilePreview,
};
use crate::models::_entities::{event_attendees, event_tags, events, profiles, tags};

const PREVIEW_LIMIT: usize = 5;

fn status_from_str(s: &str) -> AttendeeStatus {
    match s {
        "going" => AttendeeStatus::Going,
        "interested" => AttendeeStatus::Interested,
        _ => AttendeeStatus::Invited,
    }
}

async fn creator_preview(
    db: &DatabaseConnection,
    creator_id: Uuid,
) -> std::result::Result<ProfilePreview, loco_rs::Error> {
    let profile = profiles::Entity::find_by_id(creator_id)
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    Ok(profile.map_or_else(
        || ProfilePreview {
            id: creator_id.to_string(),
            name: "Unknown".to_string(),
            profile_picture: None,
        },
        |p| ProfilePreview {
            id: p.id.to_string(),
            name: p.name.clone(),
            profile_picture: p.profile_picture,
        },
    ))
}

async fn load_event_tags(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> std::result::Result<Vec<EventTagResponse>, loco_rs::Error> {
    let links = event_tags::Entity::find()
        .filter(event_tags::Column::EventId.eq(event_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = links.iter().map(|l| l.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_models = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    Ok(tag_models
        .iter()
        .map(|t| EventTagResponse {
            id: t.id.to_string(),
            name: t.name.clone(),
            scope: match t.scope.as_str() {
                "activity" => crate::controllers::migration_api::state::TagScope::Activity,
                "event" => crate::controllers::migration_api::state::TagScope::Event,
                _ => crate::controllers::migration_api::state::TagScope::Interest,
            },
        })
        .collect())
}

struct AttendeeRow {
    profile: profiles::Model,
    status: AttendeeStatus,
}

async fn load_attendee_rows(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeRow>, loco_rs::Error> {
    let attendee_links = event_attendees::Entity::find()
        .filter(event_attendees::Column::EventId.eq(event_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let profile_ids: Vec<Uuid> = attendee_links.iter().map(|a| a.profile_id).collect();
    if profile_ids.is_empty() {
        return Ok(vec![]);
    }

    let profile_models = profiles::Entity::find()
        .filter(profiles::Column::Id.is_in(profile_ids))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let mut rows: Vec<AttendeeRow> = attendee_links
        .iter()
        .filter_map(|link| {
            profile_models
                .iter()
                .find(|p| p.id == link.profile_id)
                .map(|profile| AttendeeRow {
                    profile: profile.clone(),
                    status: status_from_str(&link.status),
                })
        })
        .collect();

    rows.sort_by(|a, b| a.profile.name.cmp(&b.profile.name));
    Ok(rows)
}

pub(in crate::controllers::migration_api) async fn build_event_response(
    db: &DatabaseConnection,
    event: &events::Model,
    profile_id: &Uuid,
) -> std::result::Result<EventResponse, loco_rs::Error> {
    let attendees = load_attendee_rows(db, event.id).await?;
    let attendees_count = attendees
        .iter()
        .filter(|a| a.status == AttendeeStatus::Going)
        .count();

    let attendees_preview = attendees
        .iter()
        .filter(|a| a.status == AttendeeStatus::Going)
        .take(PREVIEW_LIMIT)
        .map(|a| ProfilePreview {
            id: a.profile.id.to_string(),
            name: a.profile.name.clone(),
            profile_picture: a.profile.profile_picture.clone(),
        })
        .collect::<Vec<_>>();

    let is_attending = attendees
        .iter()
        .any(|a| a.profile.id == *profile_id && a.status == AttendeeStatus::Going);

    let creator = creator_preview(db, event.creator_id).await?;
    let event_tags = load_event_tags(db, event.id).await?;

    Ok(EventResponse {
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
    })
}

pub(in crate::controllers::migration_api) async fn attendee_info(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeFullInfo>, loco_rs::Error> {
    let rows = load_attendee_rows(db, event_id).await?;

    let user_ids: Vec<i32> = rows.iter().map(|r| r.profile.user_id).collect();
    let users = if user_ids.is_empty() {
        vec![]
    } else {
        crate::models::_entities::users::Entity::find()
            .filter(crate::models::_entities::users::Column::Id.is_in(user_ids))
            .all(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    };

    Ok(rows
        .into_iter()
        .map(|row| {
            let user_pid = users
                .iter()
                .find(|u| u.id == row.profile.user_id)
                .map_or(uuid::Uuid::nil(), |u| u.pid);
            AttendeeFullInfo {
                id: row.profile.id.to_string(),
                user_id: user_pid.to_string(),
                name: row.profile.name.clone(),
                profile_picture: row.profile.profile_picture.clone(),
                status: row.status,
            }
        })
        .collect())
}

pub(in crate::controllers::migration_api) fn created_event_response(
    data: EventResponse,
) -> Response {
    (
        axum::http::StatusCode::CREATED,
        axum::Json(DataResponse { data }),
    )
        .into_response()
}
