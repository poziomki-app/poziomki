use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_tags::EventTag;
use crate::db::models::tags::{NewTag, Tag};
use crate::db::schema::{event_attendees, event_tags, tags};

pub(in crate::api) async fn find_or_create_event_tag_with_conn(
    conn: &mut AsyncPgConnection,
    name: String,
) -> std::result::Result<Uuid, crate::error::AppError> {
    if let Some(tag) = tags::table
        .filter(tags::scope.eq("event"))
        .filter(tags::name.eq(&name))
        .first::<Tag>(conn)
        .await
        .optional()?
    {
        return Ok(tag.id);
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
        .execute(conn)
        .await?;
    Ok(new_id)
}

pub(in crate::api) async fn load_existing_event_tag_ids(
    conn: &mut AsyncPgConnection,
    tag_ids: &[Uuid],
) -> std::result::Result<Vec<Uuid>, crate::error::AppError> {
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    tags::table
        .filter(tags::scope.eq("event"))
        .filter(tags::id.eq_any(tag_ids))
        .select(tags::id)
        .load::<Uuid>(conn)
        .await
        .map_err(Into::into)
}

pub(in crate::api) async fn sync_event_tags_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), crate::error::AppError> {
    diesel::delete(event_tags::table.filter(event_tags::event_id.eq(event_id)))
        .execute(conn)
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
            .execute(conn)
            .await?;
    }

    Ok(())
}

pub(in crate::api) async fn upsert_attendee_with_conn(
    conn: &mut AsyncPgConnection,
    event_uuid: Uuid,
    profile_id: Uuid,
    status: &str,
) -> std::result::Result<(), crate::error::AppError> {
    diesel::insert_into(event_attendees::table)
        .values(EventAttendee {
            event_id: event_uuid,
            profile_id,
            status: status.to_string(),
        })
        .on_conflict((event_attendees::event_id, event_attendees::profile_id))
        .do_update()
        .set(event_attendees::status.eq(status))
        .execute(conn)
        .await?;

    Ok(())
}
