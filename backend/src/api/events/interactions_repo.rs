use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_interactions::EventInteraction;
use crate::db::schema::event_interactions;

pub(in crate::api) const EVENT_INTERACTION_SAVED: &str = "saved";
pub(super) const EVENT_INTERACTION_JOINED: &str = "joined";

pub(super) async fn upsert_event_interaction(
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    upsert_event_interaction_with_conn(&mut conn, profile_id, event_id, kind).await
}

pub(super) async fn upsert_event_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
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

pub(super) async fn delete_event_interaction(
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    delete_event_interaction_with_conn(&mut conn, profile_id, event_id, kind).await
}

pub(super) async fn delete_event_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
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
