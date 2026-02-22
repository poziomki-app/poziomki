mod outbox;

use loco_rs::{app::AppContext, Result};

pub(crate) use outbox::{
    enqueue_matrix_event_membership_sync, enqueue_matrix_profile_avatar_sync, enqueue_otp_email,
    enqueue_upload_variants_generation, outbox_stats_snapshot, OutboxStatsSnapshot,
};

pub fn start_background_workers(ctx: &AppContext) -> Result<()> {
    outbox::maybe_start_worker(ctx);
    Ok(())
}
