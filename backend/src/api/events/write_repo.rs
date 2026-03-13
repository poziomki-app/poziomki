use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_interactions::EventInteraction;
use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::db::schema::{event_attendees, event_interactions, events};

/// Atomically check capacity, upsert the attendee, and track the interaction
/// inside a serializable transaction so that two concurrent requests cannot
/// exceed `max_attendees` and the interaction stays consistent.
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
            let status = status.clone();
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

                // Upsert attendee: delete-then-insert
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
                    status: status.clone(),
                };
                diesel::insert_into(event_attendees::table)
                    .values(&attendee)
                    .execute(conn)
                    .await?;

                // Track "joined" interaction atomically with the attendee change
                if status == "going" {
                    upsert_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
                } else {
                    delete_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
                }

                Ok(true)
            })
        })
        .await
        .map_err(Into::into)
}

pub(in crate::api) async fn insert_event_with_conn(
    conn: &mut AsyncPgConnection,
    new_event: &NewEvent,
) -> std::result::Result<Event, crate::error::AppError> {
    let inserted = diesel::insert_into(events::table)
        .values(new_event)
        .get_result::<Event>(conn)
        .await?;
    Ok(inserted)
}

pub(in crate::api) async fn update_event_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    changeset: &EventChangeset,
) -> std::result::Result<Event, crate::error::AppError> {
    let updated = diesel::update(events::table.find(event_id))
        .set(changeset)
        .get_result::<Event>(conn)
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

/// Upsert attendee status inside a transaction. When the new status is "going",
/// also records a "joined" interaction for the recommendation system.
pub(in crate::api) async fn upsert_attendee(
    event_id: Uuid,
    profile_id: Uuid,
    status: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let status = status.to_string();
    let mut conn = crate::db::conn().await?;
    conn.transaction(|conn| {
        let status = status.clone();
        Box::pin(async move {
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
                status: status.clone(),
            };
            diesel::insert_into(event_attendees::table)
                .values(&attendee)
                .execute(conn)
                .await?;

            if status == "going" {
                upsert_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
            }

            Ok::<(), crate::error::AppError>(())
        })
    })
    .await
}

pub(in crate::api) async fn delete_event_attendee(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    delete_event_attendee_with_conn(&mut conn, event_id, profile_id).await
}

pub(in crate::api) async fn delete_event_attendee_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq(profile_id)),
    )
    .execute(conn)
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

async fn upsert_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), diesel::result::Error> {
    let now = Utc::now();
    diesel::insert_into(event_interactions::table)
        .values(EventInteraction {
            profile_id,
            event_id,
            kind: kind.to_string(),
            created_at: now,
            updated_at: now,
        })
        .on_conflict((
            event_interactions::profile_id,
            event_interactions::event_id,
            event_interactions::kind,
        ))
        .do_update()
        .set(event_interactions::updated_at.eq(now))
        .execute(conn)
        .await?;
    Ok(())
}

async fn delete_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), diesel::result::Error> {
    diesel::delete(
        event_interactions::table
            .filter(event_interactions::profile_id.eq(profile_id))
            .filter(event_interactions::event_id.eq(event_id))
            .filter(event_interactions::kind.eq(kind)),
    )
    .execute(conn)
    .await?;
    Ok(())
}
