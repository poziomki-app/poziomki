use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_tags::EventTag;
use crate::db::models::tags::{NewTag, Tag};
use crate::db::schema::{event_attendees, event_tags, tags};

pub(in crate::api) async fn find_or_create_event_tag(name: String) -> Option<Uuid> {
    let mut conn = crate::db::conn().await.ok()?;

    if let Ok(Some(tag)) = tags::table
        .filter(tags::scope.eq("event"))
        .filter(tags::name.eq(&name))
        .first::<Tag>(&mut conn)
        .await
        .optional()
    {
        return Some(tag.id);
    }

    let new_id = Uuid::new_v4();
    let now = Utc::now();
    let new_tag = NewTag {
        id: new_id,
        name,
        scope: "event".to_string(),
        category: None,
        emoji: None,
        parent_id: None,
        onboarding_order: None,
        created_at: now,
        updated_at: now,
    };
    diesel::insert_into(tags::table)
        .values(&new_tag)
        .execute(&mut conn)
        .await
        .ok()
        .map(|_| new_id)
}

pub(in crate::api) async fn sync_event_tags(
    event_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    diesel::delete(event_tags::table.filter(event_tags::event_id.eq(event_id)))
        .execute(&mut conn)
        .await?;

    let new_tags: Vec<EventTag> = tag_ids
        .iter()
        .map(|tag_id| EventTag {
            event_id,
            tag_id: *tag_id,
        })
        .collect();

    if !new_tags.is_empty() {
        diesel::insert_into(event_tags::table)
            .values(&new_tags)
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

pub(in crate::api) async fn upsert_attendee(
    event_uuid: Uuid,
    profile_id: Uuid,
    status: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_uuid))
            .filter(event_attendees::profile_id.eq(profile_id)),
    )
    .execute(&mut conn)
    .await?;

    let attendee = EventAttendee {
        event_id: event_uuid,
        profile_id,
        status: status.to_string(),
    };
    diesel::insert_into(event_attendees::table)
        .values(&attendee)
        .execute(&mut conn)
        .await?;

    Ok(())
}
