use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::events::Event;
use crate::db::schema::{event_interactions, events};

pub(in crate::api) async fn list_upcoming_events(
    now: DateTime<Utc>,
    limit: i64,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let models = events::table
        .filter(events::starts_at.ge(now))
        .order(events::starts_at.asc())
        .limit(limit)
        .load::<Event>(&mut conn)
        .await?;
    Ok(models)
}

pub(in crate::api) async fn list_events_by_creator(
    creator_id: Uuid,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let models = events::table
        .filter(events::creator_id.eq(creator_id))
        .order(events::starts_at.desc())
        .load::<Event>(&mut conn)
        .await?;
    Ok(models)
}

pub(in crate::api) async fn list_saved_events(
    profile_id: Uuid,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let models = events::table
        .inner_join(
            event_interactions::table.on(event_interactions::event_id
                .eq(events::id)
                .and(event_interactions::profile_id.eq(profile_id))
                .and(event_interactions::kind.eq("saved"))),
        )
        .order(event_interactions::created_at.desc())
        .select(events::all_columns)
        .load::<Event>(&mut conn)
        .await?;
    Ok(models)
}

pub(in crate::api) async fn find_event(
    event_id: Uuid,
) -> std::result::Result<Option<Event>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let model = events::table
        .find(event_id)
        .first::<Event>(&mut conn)
        .await
        .optional()?;
    Ok(model)
}
