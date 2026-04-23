use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::RunQueryDsl;
use std::time::Duration;

use crate::db::schema::{otp_codes, sessions};

const CLEANUP_INTERVAL_SECS: u64 = 3600;

/// How long processed `job_outbox` rows are retained. Long enough for
/// operators to triage a spike from an audit log / metrics snapshot,
/// short enough that message-body snapshots embedded in moderation-scan
/// payloads don't accumulate into a long-lived shadow copy of chat
/// content. 7 days is the compromise.
const OUTBOX_PROCESSED_RETENTION_DAYS: i64 = 7;

pub(super) async fn run_cleanup_loop() {
    tracing::info!("cleanup loop started (interval: {CLEANUP_INTERVAL_SECS}s)");
    loop {
        if let Err(error) = purge_expired_rows().await {
            tracing::warn!(%error, "session/otp cleanup failed");
        }
        if let Err(error) = purge_processed_outbox_jobs().await {
            tracing::warn!(%error, "processed outbox cleanup failed");
        }
        tokio::time::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;
    }
}

async fn purge_expired_rows() -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let now = chrono::Utc::now();

    let sessions_deleted = diesel::delete(sessions::table.filter(sessions::expires_at.le(now)))
        .execute(&mut conn)
        .await?;

    let otps_deleted = diesel::delete(otp_codes::table.filter(otp_codes::expires_at.le(now)))
        .execute(&mut conn)
        .await?;

    if sessions_deleted > 0 || otps_deleted > 0 {
        tracing::info!(sessions_deleted, otps_deleted, "expired rows purged");
    }

    Ok(())
}

/// Purge long-since-processed outbox rows. Moderation-scan payloads carry
/// a snapshot of the message body (so the scan can't be bypassed by a
/// quick edit/delete); the retention window bounds how long those body
/// snapshots live past their processing time.
async fn purge_processed_outbox_jobs() -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let deleted = diesel::sql_query(
        r"
        DELETE FROM job_outbox
        WHERE processed_at IS NOT NULL
          AND processed_at < NOW() - make_interval(days => $1)
        ",
    )
    .bind::<BigInt, _>(OUTBOX_PROCESSED_RETENTION_DAYS)
    .execute(&mut conn)
    .await?;

    if deleted > 0 {
        tracing::info!(
            outbox_jobs_deleted = deleted,
            "processed outbox rows purged"
        );
    }

    Ok(())
}
