use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::db::schema::{event_attendees, events};

/// Atomically check capacity and upsert the attendee inside a serializable
/// transaction so that two concurrent requests cannot exceed `max_attendees`.
/// Returns `true` when the upsert succeeded, `false` when the event is full.
pub(in crate::api) async fn check_capacity_and_upsert(
    event_id: Uuid,
    profile_id: Uuid,
    status: &str,
    max_attendees: Option<i32>,
    already_going: bool,
) -> std::result::Result<bool, crate::error::AppError> {
    let status = status.to_string();
    let mut conn = crate::db::conn().await?;

    conn.build_transaction()
        .serializable()
        .run(|conn| {
            Box::pin(async move {
                // Capacity gate: only enforce when switching to "going"
                if status == "going" && !already_going {
                    if let Some(max) = max_attendees {
                        let current_going = event_attendees::table
                            .filter(event_attendees::event_id.eq(event_id))
                            .filter(event_attendees::status.eq("going"))
                            .count()
                            .get_result::<i64>(conn)
                            .await?;
                        if current_going >= i64::from(max) {
                            return Ok::<bool, diesel::result::Error>(false);
                        }
                    }
                }

                // Upsert: delete-then-insert
                diesel::delete(
                    event_attendees::table
                        .filter(event_attendees::event_id.eq(event_id))
                        .filter(event_attendees::profile_id.eq(profile_id)),
                )
                .execute(conn)
                .await?;

                let attendee = EventAttendee {
                    event_id,
                    profile_id,
                    status,
                };
                diesel::insert_into(event_attendees::table)
                    .values(&attendee)
                    .execute(conn)
                    .await?;

                Ok(true)
            })
        })
        .await
        .map_err(Into::into)
}

pub(in crate::api) async fn insert_event(
    new_event: &NewEvent,
) -> std::result::Result<Event, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let inserted = diesel::insert_into(events::table)
        .values(new_event)
        .get_result::<Event>(&mut conn)
        .await?;
    Ok(inserted)
}

pub(in crate::api) async fn update_event(
    event_id: Uuid,
    changeset: &EventChangeset,
) -> std::result::Result<Event, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let updated = diesel::update(events::table.find(event_id))
        .set(changeset)
        .get_result::<Event>(&mut conn)
        .await?;
    Ok(updated)
}

pub(in crate::api) async fn delete_event(
    event_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::delete(events::table.find(event_id))
        .execute(&mut conn)
        .await?;
    Ok(())
}

pub(in crate::api) async fn delete_event_attendee(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq(profile_id)),
    )
    .execute(&mut conn)
    .await?;
    Ok(())
}

pub(in crate::api) async fn find_attendee_status(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<Option<String>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let status = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::profile_id.eq(profile_id))
        .select(event_attendees::status)
        .first::<String>(&mut conn)
        .await
        .optional()?;
    Ok(status)
}

pub(in crate::api) async fn load_attendee(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<Option<EventAttendee>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let attendee = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::profile_id.eq(profile_id))
        .first::<EventAttendee>(&mut conn)
        .await
        .optional()?;
    Ok(attendee)
}
