mod outbox_dispatch;
mod outbox_worker;

use crate::app::AppContext;
type Result<T> = crate::error::AppResult<T>;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};

static OUTBOX_WORKER_STARTED: AtomicBool = AtomicBool::new(false);
pub(super) const OUTBOX_LOCK_TIMEOUT_SECS: i64 = 300;
const OUTBOX_DEFAULT_MAX_ATTEMPTS: i32 = 10;
pub(super) const OUTBOX_WORKER_HEARTBEAT_PATH: &str = "/tmp/poziomki-outbox-worker-heartbeat";

#[derive(Debug, Clone)]
pub(super) struct OutboxJob {
    pub(super) id: String,
    pub(super) topic: String,
    pub(super) payload_json: String,
    pub(super) attempts: i32,
    pub(super) max_attempts: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutboxStatsSnapshot {
    pub pending_jobs: i64,
    pub ready_jobs: i64,
    pub retrying_jobs: i64,
    pub inflight_jobs: i64,
    pub failed_jobs: i64,
    pub exhausted_jobs: i64,
    pub processed_jobs_24h: i64,
    pub oldest_ready_job_age_seconds: i64,
    pub oldest_pending_job_age_seconds: i64,
    pub last_processed_at: Option<String>,
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub async fn enqueue_otp_email(to: &str, code: &str) -> Result<()> {
    let payload = json!({
        "to": to,
        "code": code,
    })
    .to_string();

    enqueue_job(outbox_dispatch::OUTBOX_TOPIC_OTP_EMAIL, payload).await
}

pub async fn enqueue_matrix_profile_avatar_sync(
    user_pid: &uuid::Uuid,
    profile_picture_filename: Option<&str>,
) -> Result<()> {
    let payload = json!({
        "user_pid": user_pid.to_string(),
        "profile_picture_filename": profile_picture_filename,
    })
    .to_string();

    enqueue_job(
        outbox_dispatch::OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC,
        payload,
    )
    .await
}

pub async fn enqueue_matrix_event_membership_sync(
    event_id: &uuid::Uuid,
    profile_id: &uuid::Uuid,
    leave: bool,
) -> Result<()> {
    let payload = json!({
        "event_id": event_id.to_string(),
        "profile_id": profile_id.to_string(),
        "leave": leave,
    })
    .to_string();

    enqueue_job(
        outbox_dispatch::OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC,
        payload,
    )
    .await
}

pub async fn enqueue_upload_variants_generation(upload_id: &uuid::Uuid) -> Result<()> {
    let payload = json!({
        "upload_id": upload_id.to_string(),
    })
    .to_string();

    enqueue_job(
        outbox_dispatch::OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION,
        payload,
    )
    .await
}

async fn enqueue_job(topic: &str, payload: String) -> Result<()> {
    let mut conn = crate::db::conn().await?;

    diesel::sql_query(
        r"
        INSERT INTO job_outbox (
            id,
            topic,
            payload,
            attempts,
            max_attempts,
            available_at,
            created_at,
            updated_at
        )
        VALUES ($1::uuid, $2, $3::jsonb, 0, $4, NOW(), NOW(), NOW())
        ",
    )
    .bind::<Text, _>(uuid::Uuid::new_v4().to_string())
    .bind::<Text, _>(topic)
    .bind::<Text, _>(payload)
    .bind::<Integer, _>(OUTBOX_DEFAULT_MAX_ATTEMPTS)
    .execute(&mut conn)
    .await?;
    Ok(())
}

pub fn maybe_start_worker(_ctx: &AppContext) {
    if !env_truthy("OUTBOX_WORKER_ENABLED") && std::env::var("OUTBOX_WORKER_ENABLED").is_ok() {
        tracing::info!("Outbox worker disabled via OUTBOX_WORKER_ENABLED");
        return;
    }

    if OUTBOX_WORKER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    tokio::spawn(async move {
        outbox_worker::run_worker_loop().await;
    });
    tracing::info!("Outbox worker started");
}

#[derive(Debug, QueryableByName)]
struct OutboxStatsRow {
    #[diesel(sql_type = BigInt)]
    pending_jobs: i64,
    #[diesel(sql_type = BigInt)]
    ready_jobs: i64,
    #[diesel(sql_type = BigInt)]
    retrying_jobs: i64,
    #[diesel(sql_type = BigInt)]
    inflight_jobs: i64,
    #[diesel(sql_type = BigInt)]
    failed_jobs: i64,
    #[diesel(sql_type = BigInt)]
    exhausted_jobs: i64,
    #[diesel(sql_type = BigInt)]
    processed_jobs_24h: i64,
    #[diesel(sql_type = BigInt)]
    oldest_ready_job_age_seconds: i64,
    #[diesel(sql_type = BigInt)]
    oldest_pending_job_age_seconds: i64,
    #[diesel(sql_type = Nullable<Text>)]
    last_processed_at: Option<String>,
}

pub async fn outbox_stats_snapshot() -> Result<OutboxStatsSnapshot> {
    let mut conn = crate::db::conn().await?;

    let row = diesel::sql_query(
        r"
        SELECT
            COUNT(*) FILTER (
                WHERE processed_at IS NULL AND failed_at IS NULL
            )::bigint AS pending_jobs,
            COUNT(*) FILTER (
                WHERE processed_at IS NULL
                  AND failed_at IS NULL
                  AND available_at <= NOW()
                  AND (
                    locked_at IS NULL
                    OR locked_at <= NOW() - make_interval(secs => $1)
                  )
            )::bigint AS ready_jobs,
            COUNT(*) FILTER (
                WHERE processed_at IS NULL
                  AND failed_at IS NULL
                  AND attempts > 0
            )::bigint AS retrying_jobs,
            COUNT(*) FILTER (
                WHERE processed_at IS NULL
                  AND failed_at IS NULL
                  AND locked_at IS NOT NULL
                  AND locked_at > NOW() - make_interval(secs => $1)
            )::bigint AS inflight_jobs,
            COUNT(*) FILTER (WHERE failed_at IS NOT NULL)::bigint AS failed_jobs,
            COUNT(*) FILTER (
                WHERE failed_at IS NOT NULL
                  AND attempts >= max_attempts
            )::bigint AS exhausted_jobs,
            COUNT(*) FILTER (
                WHERE processed_at IS NOT NULL
                  AND processed_at > NOW() - INTERVAL '24 hours'
            )::bigint AS processed_jobs_24h,
            COALESCE(
                FLOOR(EXTRACT(EPOCH FROM (
                    NOW() - MIN(created_at) FILTER (
                        WHERE processed_at IS NULL
                          AND failed_at IS NULL
                          AND available_at <= NOW()
                    )
                )))::bigint,
                0
            ) AS oldest_ready_job_age_seconds,
            COALESCE(
                FLOOR(EXTRACT(EPOCH FROM (
                    NOW() - MIN(created_at) FILTER (
                        WHERE processed_at IS NULL
                          AND failed_at IS NULL
                    )
                )))::bigint,
                0
            ) AS oldest_pending_job_age_seconds,
            MAX(processed_at)::text AS last_processed_at
        FROM job_outbox
        ",
    )
    .bind::<BigInt, _>(OUTBOX_LOCK_TIMEOUT_SECS)
    .get_result::<OutboxStatsRow>(&mut conn)
    .await?;

    Ok(OutboxStatsSnapshot {
        pending_jobs: row.pending_jobs,
        ready_jobs: row.ready_jobs,
        retrying_jobs: row.retrying_jobs,
        inflight_jobs: row.inflight_jobs,
        failed_jobs: row.failed_jobs,
        exhausted_jobs: row.exhausted_jobs,
        processed_jobs_24h: row.processed_jobs_24h,
        oldest_ready_job_age_seconds: row.oldest_ready_job_age_seconds,
        oldest_pending_job_age_seconds: row.oldest_pending_job_age_seconds,
        last_processed_at: row.last_processed_at,
    })
}
