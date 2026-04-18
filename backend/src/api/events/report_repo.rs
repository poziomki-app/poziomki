use diesel::prelude::*;
use diesel::OptionalExtension;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::reports::NewReport;
use crate::db::schema::{events, reports};

/// Insert an event report. Caller must supply a connection already inside
/// a viewer-scoped transaction.
///
/// Returns `None` when the event doesn't exist, `Some(true)` when the
/// report was inserted, and `Some(false)` when a duplicate already exists.
pub(in crate::api) async fn insert_event_report(
    conn: &mut AsyncPgConnection,
    reporter_id: Uuid,
    event_id: Uuid,
    reason: String,
    description: Option<String>,
) -> std::result::Result<Option<bool>, crate::error::AppError> {
    let exists = events::table
        .find(event_id)
        .select(events::id)
        .first::<Uuid>(conn)
        .await
        .optional()?;

    if exists.is_none() {
        return Ok(None);
    }

    let new = NewReport {
        reporter_id,
        target_type: "event".to_string(),
        target_id: event_id,
        reason,
        description,
    };

    let rows = diesel::insert_into(reports::table)
        .values(&new)
        .on_conflict_do_nothing()
        .execute(conn)
        .await?;

    Ok(Some(rows > 0))
}
