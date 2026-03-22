mod cleanup;
pub(crate) mod outbox;

use crate::app::AppContext;

use crate::error::AppResult;
pub(crate) use outbox::{
    enqueue_chat_membership_sync, enqueue_otp_email, enqueue_upload_variants_generation,
    outbox_stats_snapshot, OutboxStatsSnapshot,
};

pub fn start_background_workers(ctx: &AppContext) -> AppResult<()> {
    outbox::maybe_start_worker(ctx);
    tokio::spawn(async { cleanup::run_cleanup_loop().await });
    Ok(())
}
