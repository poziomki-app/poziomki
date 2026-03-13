use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::db::schema::{event_attendees, events};

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
