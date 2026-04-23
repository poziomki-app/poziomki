use diesel::deserialize::QueryableByName;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel::OptionalExtension;
use diesel_async::RunQueryDsl;
use std::fs;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use super::outbox_dispatch::{
    dispatch_job, mark_job_done, mark_job_failed, OUTBOX_TOPIC_MODERATION_SCAN,
};
use super::{
    OutboxJob, MODERATION_WORKER_HEARTBEAT_PATH, OUTBOX_LOCK_TIMEOUT_SECS,
    OUTBOX_WORKER_HEARTBEAT_PATH,
};

/// Selects which subset of `job_outbox` a worker loop will claim.
///
/// Split so moderation scans (high-volume, fast CPU work) don't share a
/// FIFO and an inter-job sleep with email/upload/membership jobs.
#[derive(Clone, Copy)]
enum JobKind {
    /// Every topic except `moderation_scan`. Preserves the historical
    /// pacing (100 ms between jobs, 2 s when idle).
    General,
    /// Only `moderation_scan`. Tight inner loop — no inter-job sleep
    /// when jobs are available.
    Moderation,
}

impl JobKind {
    const fn label(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Moderation => "moderation",
        }
    }

    const fn active_sleep(self) -> Duration {
        match self {
            Self::General => Duration::from_millis(100),
            // Moderation is CPU-bound; at ~10 ms per job an inter-job pause
            // just caps throughput for no benefit. Yield but don't sleep.
            Self::Moderation => Duration::ZERO,
        }
    }

    const fn idle_sleep(self) -> Duration {
        match self {
            Self::General => Duration::from_secs(2),
            Self::Moderation => Duration::from_secs(1),
        }
    }
}

pub(super) async fn run_general_worker_loop() {
    run_worker_loop(JobKind::General).await;
}

pub(super) async fn run_moderation_worker_loop() {
    run_worker_loop(JobKind::Moderation).await;
}

async fn run_worker_loop(kind: JobKind) {
    loop {
        // Each loop writes its own heartbeat file. Healthcheck requires
        // both to be fresh, so a wedged moderation loop marks the worker
        // container unhealthy even if the general loop is still pumping.
        write_heartbeat(heartbeat_path(kind));
        let processed = match process_one_job(kind).await {
            Ok(processed) => processed,
            Err(error) => {
                tracing::error!(%error, worker = kind.label(), "outbox worker loop error");
                false
            }
        };

        let sleep_for = if processed {
            kind.active_sleep()
        } else {
            kind.idle_sleep()
        };
        if sleep_for.is_zero() {
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(sleep_for).await;
        }
    }
}

pub(super) async fn run_metrics_loop() {
    loop {
        match super::outbox_stats_snapshot().await {
            Ok(snapshot) => crate::telemetry::update_outbox_metrics(&snapshot),
            Err(error) => tracing::warn!(%error, "failed to refresh outbox metrics"),
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

const fn heartbeat_path(kind: JobKind) -> &'static str {
    match kind {
        JobKind::General => OUTBOX_WORKER_HEARTBEAT_PATH,
        JobKind::Moderation => MODERATION_WORKER_HEARTBEAT_PATH,
    }
}

fn write_heartbeat(path: &'static str) {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_else(|_| "0".to_string(), |d| d.as_secs().to_string());

    if let Err(error) = fs::write(path, now_epoch) {
        tracing::warn!(%error, path, "failed to write worker heartbeat");
    }
}

#[derive(Debug, QueryableByName)]
struct OutboxJobRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    topic: String,
    #[diesel(sql_type = Text)]
    payload_json: String,
    #[diesel(sql_type = Integer)]
    attempts: i32,
    #[diesel(sql_type = Integer)]
    max_attempts: i32,
}

#[tracing::instrument(skip_all, fields(worker = kind.label()))]
async fn process_one_job(kind: JobKind) -> std::result::Result<bool, String> {
    let Some(job) = claim_next_job(kind).await.map_err(|e| e.to_string())? else {
        return Ok(false);
    };

    let result = dispatch_job(&job).await;
    match result {
        Ok(()) => {
            mark_job_done(&job.id).await.map_err(|e| e.to_string())?;
            crate::telemetry::record_outbox_job_result(&job.topic, "success");
        }
        Err(ref error) => {
            mark_job_failed(&job, error)
                .await
                .map_err(|e| e.to_string())?;
            let is_terminal = job.attempts >= job.max_attempts;
            if is_terminal {
                crate::telemetry::record_outbox_job_result(&job.topic, "exhausted");
                tracing::error!(%error, job_id = %job.id, job_topic = %job.topic, attempts = job.attempts, max_attempts = job.max_attempts, "job permanently failed");
            } else {
                crate::telemetry::record_outbox_job_result(&job.topic, "retry");
                tracing::warn!(%error, job_id = %job.id, job_topic = %job.topic, attempts = job.attempts, max_attempts = job.max_attempts, "job dispatch failed, will retry");
            }
        }
    }

    Ok(true)
}

#[tracing::instrument(skip_all, fields(worker = kind.label()))]
async fn claim_next_job(
    kind: JobKind,
) -> std::result::Result<Option<OutboxJob>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    // Two workers share the same table; `FOR UPDATE SKIP LOCKED` keeps
    // them from fighting. The topic predicate below partitions the work:
    // General skips moderation_scan; Moderation claims only that topic.
    let (sql_fragment, topic_bind) = match kind {
        JobKind::General => (
            "WHERE processed_at IS NULL
              AND failed_at IS NULL
              AND available_at <= NOW()
              AND topic <> $2
              AND (
                locked_at IS NULL
                OR locked_at <= NOW() - make_interval(secs => $1)
              )",
            OUTBOX_TOPIC_MODERATION_SCAN,
        ),
        JobKind::Moderation => (
            "WHERE processed_at IS NULL
              AND failed_at IS NULL
              AND available_at <= NOW()
              AND topic = $2
              AND (
                locked_at IS NULL
                OR locked_at <= NOW() - make_interval(secs => $1)
              )",
            OUTBOX_TOPIC_MODERATION_SCAN,
        ),
    };

    let sql = format!(
        "WITH picked AS (
            SELECT id
            FROM job_outbox
            {sql_fragment}
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE job_outbox AS j
        SET locked_at = NOW(),
            attempts = j.attempts + 1,
            updated_at = NOW()
        FROM picked
        WHERE j.id = picked.id
        RETURNING j.id::text AS id,
                  j.topic,
                  j.payload::text AS payload_json,
                  j.attempts,
                  j.max_attempts"
    );

    let row = diesel::sql_query(sql)
        .bind::<BigInt, _>(OUTBOX_LOCK_TIMEOUT_SECS)
        .bind::<Text, _>(topic_bind)
        .get_result::<OutboxJobRow>(&mut conn)
        .await
        .optional()?;

    Ok(row.map(|r| OutboxJob {
        id: r.id,
        topic: r.topic,
        payload_json: r.payload_json,
        attempts: r.attempts,
        max_attempts: r.max_attempts,
    }))
}
