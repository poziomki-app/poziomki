use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::reports::NewReport;
use crate::db::schema::reports;

/// Insert an event report. Returns `true` if inserted, `false` if a duplicate already exists.
pub(in crate::api) async fn insert_event_report(
    reporter_id: Uuid,
    event_id: Uuid,
    reason: String,
    description: Option<String>,
) -> std::result::Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

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
        .execute(&mut conn)
        .await?;

    Ok(rows > 0)
}
