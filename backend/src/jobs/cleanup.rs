use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::time::Duration;

use crate::db::schema::{otp_codes, sessions};

const CLEANUP_INTERVAL_SECS: u64 = 3600;

pub(super) async fn run_cleanup_loop() {
    loop {
        if let Err(error) = purge_expired_rows().await {
            tracing::warn!(%error, "session/otp cleanup failed");
        }
        tokio::time::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;
    }
}

async fn purge_expired_rows() -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let now = chrono::Utc::now();

    let sessions_deleted = diesel::delete(sessions::table.filter(sessions::expires_at.lt(now)))
        .execute(&mut conn)
        .await?;

    let otps_deleted = diesel::delete(otp_codes::table.filter(otp_codes::expires_at.lt(now)))
        .execute(&mut conn)
        .await?;

    if sessions_deleted > 0 || otps_deleted > 0 {
        tracing::info!(sessions_deleted, otps_deleted, "expired rows purged");
    }

    Ok(())
}
