use diesel::deserialize::QueryableByName;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel::OptionalExtension;
use diesel_async::RunQueryDsl;
use std::fs;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use super::outbox_dispatch::{dispatch_job, mark_job_done, mark_job_failed};
use super::{OutboxJob, OUTBOX_LOCK_TIMEOUT_SECS, OUTBOX_WORKER_HEARTBEAT_PATH};

pub(super) async fn run_worker_loop() {
    loop {
        write_worker_heartbeat();
        let processed = match process_one_job().await {
            Ok(processed) => processed,
            Err(error) => {
                tracing::error!(%error, "outbox worker loop error");
                false
            }
        };

        let sleep_for = if processed {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(2)
        };
        tokio::time::sleep(sleep_for).await;
    }
}

fn write_worker_heartbeat() {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_else(|_| "0".to_string(), |d| d.as_secs().to_string());

    if let Err(error) = fs::write(OUTBOX_WORKER_HEARTBEAT_PATH, now_epoch) {
        tracing::warn!(%error, path = OUTBOX_WORKER_HEARTBEAT_PATH, "failed to write worker heartbeat");
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

#[tracing::instrument(skip_all)]
async fn process_one_job() -> std::result::Result<bool, String> {
    let Some(job) = claim_next_job().await.map_err(|e| e.to_string())? else {
        return Ok(false);
    };

    let result = dispatch_job(&job).await;
    match result {
        Ok(()) => {
            mark_job_done(&job.id).await.map_err(|e| e.to_string())?;
        }
        Err(ref error) => {
            mark_job_failed(&job, error)
                .await
                .map_err(|e| e.to_string())?;
            let is_terminal = job.attempts >= job.max_attempts;
            if is_terminal {
                tracing::error!(%error, job_id = %job.id, job_topic = %job.topic, attempts = job.attempts, max_attempts = job.max_attempts, "job permanently failed");
            } else {
                tracing::warn!(%error, job_id = %job.id, job_topic = %job.topic, attempts = job.attempts, max_attempts = job.max_attempts, "job dispatch failed, will retry");
            }
        }
    }

    Ok(true)
}

#[tracing::instrument(skip_all)]
async fn claim_next_job() -> std::result::Result<Option<OutboxJob>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let row = diesel::sql_query(
        r"
        WITH picked AS (
            SELECT id
            FROM job_outbox
            WHERE processed_at IS NULL
              AND failed_at IS NULL
              AND available_at <= NOW()
              AND (
                locked_at IS NULL
                OR locked_at <= NOW() - make_interval(secs => $1)
              )
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
                  j.max_attempts
        ",
    )
    .bind::<BigInt, _>(OUTBOX_LOCK_TIMEOUT_SECS)
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
