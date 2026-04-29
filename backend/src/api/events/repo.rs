use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::events::Event;
use crate::db::schema::{event_interactions, events};

pub(in crate::api) async fn list_upcoming_events(
    conn: &mut AsyncPgConnection,
    now: DateTime<Utc>,
    limit: i64,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let models = events::table
        .filter(
            events::ends_at
                .is_null()
                .and(events::starts_at.ge(now))
                .or(events::ends_at.ge(now)),
        )
        .order(events::starts_at.asc())
        .limit(limit)
        .load::<Event>(conn)
        .await?;
    Ok(models)
}

// "My events" = events the viewer created OR joined as an attendee. The
// mobile client uses this list to hydrate event-room covers in Wiadomości
// and the chat header. Restricting to creator alone leaves attendees with
// no cover until they manually open the event detail screen.
pub(in crate::api) async fn list_my_events(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let joined_event_ids = event_interactions::table
        .filter(event_interactions::profile_id.eq(profile_id))
        .filter(event_interactions::kind.eq(super::EVENT_INTERACTION_JOINED))
        .select(event_interactions::event_id);
    let models = events::table
        .filter(
            events::creator_id
                .eq(profile_id)
                .or(events::id.eq_any(joined_event_ids)),
        )
        .order(events::starts_at.desc())
        .load::<Event>(conn)
        .await?;
    Ok(models)
}

pub(in crate::api) async fn list_saved_events(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<Event>, crate::error::AppError> {
    let models = events::table
        .inner_join(
            event_interactions::table.on(event_interactions::event_id
                .eq(events::id)
                .and(event_interactions::profile_id.eq(profile_id))
                .and(event_interactions::kind.eq(super::EVENT_INTERACTION_SAVED))),
        )
        .order(event_interactions::created_at.desc())
        .select(events::all_columns)
        .load::<Event>(conn)
        .await?;
    Ok(models)
}

pub(in crate::api) async fn find_event(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
) -> std::result::Result<Option<Event>, crate::error::AppError> {
    let model = events::table
        .find(event_id)
        .first::<Event>(conn)
        .await
        .optional()?;
    Ok(model)
}
