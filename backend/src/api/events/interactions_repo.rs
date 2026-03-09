use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::event_interactions::EventInteraction;
use crate::db::schema::event_interactions;

pub(super) const EVENT_INTERACTION_SAVED: &str = "saved";
pub(super) const EVENT_INTERACTION_JOINED: &str = "joined";

pub(super) async fn upsert_event_interaction(
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let now = Utc::now();

    diesel::delete(
        event_interactions::table
            .filter(event_interactions::profile_id.eq(profile_id))
            .filter(event_interactions::event_id.eq(event_id))
            .filter(event_interactions::kind.eq(kind)),
    )
    .execute(&mut conn)
    .await?;

    diesel::insert_into(event_interactions::table)
        .values(EventInteraction {
            profile_id,
            event_id,
            kind: kind.to_string(),
            created_at: now,
            updated_at: now,
        })
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub(super) async fn delete_event_interaction(
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::delete(
        event_interactions::table
            .filter(event_interactions::profile_id.eq(profile_id))
            .filter(event_interactions::event_id.eq(event_id))
            .filter(event_interactions::kind.eq(kind)),
    )
    .execute(&mut conn)
    .await?;
    Ok(())
}
