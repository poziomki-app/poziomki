use diesel::prelude::*;
use diesel::OptionalExtension;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::reports::NewReport;
use crate::db::schema::{conversations, reports};

/// Insert a conversation report. Caller must supply a connection that is
/// already inside a transaction (wrap via `db::with_viewer_tx`).
///
/// Returns `None` if the conversation doesn't exist, `Some(true)` if the
/// report was inserted, and `Some(false)` if a duplicate already exists.
pub(in crate::api) async fn insert_conversation_report(
    conn: &mut AsyncPgConnection,
    reporter_id: Uuid,
    conversation_id: Uuid,
    reason: String,
    description: Option<String>,
) -> std::result::Result<Option<bool>, crate::error::AppError> {
    let exists = conversations::table
        .find(conversation_id)
        .select(conversations::id)
        .first::<Uuid>(conn)
        .await
        .optional()?;

    if exists.is_none() {
        return Ok(None);
    }

    let new = NewReport {
        reporter_id,
        target_type: "conversation".to_string(),
        target_id: conversation_id,
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
