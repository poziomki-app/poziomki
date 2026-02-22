use crate::app::AppContext;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};

static OUTBOX_WORKER_STARTED: AtomicBool = AtomicBool::new(false);
const OUTBOX_TOPIC_OTP_EMAIL: &str = "otp_email_send";
const OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC: &str = "matrix_profile_avatar_sync";
const OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC: &str = "matrix_event_membership_sync";
const OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION: &str = "upload_variants_generation";
const OUTBOX_LOCK_TIMEOUT_SECS: i64 = 300;
const OUTBOX_DEFAULT_MAX_ATTEMPTS: i32 = 10;
const OUTBOX_WORKER_HEARTBEAT_PATH: &str = "/tmp/poziomki-outbox-worker-heartbeat";

#[derive(Debug, Clone)]
struct OutboxJob {
    id: String,
    topic: String,
    payload_json: String,
    attempts: i32,
    max_attempts: i32,
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

#[derive(Debug, Deserialize)]
struct OtpEmailJobPayload {
    to: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct MatrixProfileAvatarSyncJobPayload {
    user_pid: String,
    profile_picture_filename: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MatrixEventMembershipSyncJobPayload {
    event_id: String,
    profile_id: String,
    leave: bool,
}

#[derive(Debug, Deserialize)]
struct UploadVariantsGenerationJobPayload {
    upload_id: String,
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub async fn enqueue_otp_email(db: &DatabaseConnection, to: &str, code: &str) -> AppResult<()> {
    let payload = json!({
        "to": to,
        "code": code,
    })
    .to_string();

    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
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
        VALUES ($1, $2, $3::jsonb, 0, $4, NOW(), NOW(), NOW())
        ",
        vec![
            uuid::Uuid::new_v4().to_string().into(),
            OUTBOX_TOPIC_OTP_EMAIL.to_string().into(),
            payload.into(),
            OUTBOX_DEFAULT_MAX_ATTEMPTS.into(),
        ],
    );

    db.execute(stmt)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Any(e.into()))
}

pub async fn enqueue_matrix_profile_avatar_sync(
    db: &DatabaseConnection,
    user_pid: &uuid::Uuid,
    profile_picture_filename: Option<&str>,
) -> AppResult<()> {
    let payload = json!({
        "user_pid": user_pid.to_string(),
        "profile_picture_filename": profile_picture_filename,
    })
    .to_string();

    enqueue_job(db, OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC, payload).await
}

pub async fn enqueue_matrix_event_membership_sync(
    db: &DatabaseConnection,
    event_id: &uuid::Uuid,
    profile_id: &uuid::Uuid,
    leave: bool,
) -> AppResult<()> {
    let payload = json!({
        "event_id": event_id.to_string(),
        "profile_id": profile_id.to_string(),
        "leave": leave,
    })
    .to_string();

    enqueue_job(db, OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC, payload).await
}

pub async fn enqueue_upload_variants_generation(
    db: &DatabaseConnection,
    upload_id: &uuid::Uuid,
) -> AppResult<()> {
    let payload = json!({
        "upload_id": upload_id.to_string(),
    })
    .to_string();

    enqueue_job(db, OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION, payload).await
}

async fn enqueue_job(db: &DatabaseConnection, topic: &str, payload: String) -> AppResult<()> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
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
        VALUES ($1, $2, $3::jsonb, 0, $4, NOW(), NOW(), NOW())
        ",
        vec![
            uuid::Uuid::new_v4().to_string().into(),
            topic.to_string().into(),
            payload.into(),
            OUTBOX_DEFAULT_MAX_ATTEMPTS.into(),
        ],
    );

    db.execute(stmt)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Any(e.into()))
}

pub fn maybe_start_worker(ctx: &AppContext) {
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

    let db = ctx.db.clone();
    tokio::spawn(async move {
        run_worker_loop(db).await;
    });
    tracing::info!("Outbox worker started");
}

pub async fn outbox_stats_snapshot(db: &DatabaseConnection) -> AppResult<OutboxStatsSnapshot> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
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
        vec![OUTBOX_LOCK_TIMEOUT_SECS.into()],
    );

    let row = db
        .query_one(stmt)
        .await
        .map_err(|e| AppError::Any(e.into()))?
        .ok_or_else(|| AppError::Message("outbox stats query returned no rows".to_string()))?;

    Ok(OutboxStatsSnapshot {
        pending_jobs: row
            .try_get("", "pending_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        ready_jobs: row
            .try_get("", "ready_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        retrying_jobs: row
            .try_get("", "retrying_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        inflight_jobs: row
            .try_get("", "inflight_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        failed_jobs: row
            .try_get("", "failed_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        exhausted_jobs: row
            .try_get("", "exhausted_jobs")
            .map_err(|e| AppError::Any(e.into()))?,
        processed_jobs_24h: row
            .try_get("", "processed_jobs_24h")
            .map_err(|e| AppError::Any(e.into()))?,
        oldest_ready_job_age_seconds: row
            .try_get("", "oldest_ready_job_age_seconds")
            .map_err(|e| AppError::Any(e.into()))?,
        oldest_pending_job_age_seconds: row
            .try_get("", "oldest_pending_job_age_seconds")
            .map_err(|e| AppError::Any(e.into()))?,
        last_processed_at: row
            .try_get("", "last_processed_at")
            .map_err(|e| AppError::Any(e.into()))?,
    })
}

async fn run_worker_loop(db: DatabaseConnection) {
    loop {
        write_worker_heartbeat();
        let processed = match process_one_job(&db).await {
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

async fn process_one_job(db: &DatabaseConnection) -> std::result::Result<bool, sea_orm::DbErr> {
    let Some(job) = claim_next_job(db).await? else {
        return Ok(false);
    };

    let result = dispatch_job(db, &job).await;
    match result {
        Ok(()) => {
            mark_job_done(db, &job.id).await?;
        }
        Err(error) => {
            mark_job_failed(db, &job, &error).await?;
        }
    }

    Ok(true)
}

async fn claim_next_job(
    db: &DatabaseConnection,
) -> std::result::Result<Option<OutboxJob>, sea_orm::DbErr> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
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
        vec![OUTBOX_LOCK_TIMEOUT_SECS.into()],
    );

    let Some(row) = db.query_one(stmt).await? else {
        return Ok(None);
    };

    Ok(Some(OutboxJob {
        id: row.try_get("", "id")?,
        topic: row.try_get("", "topic")?,
        payload_json: row.try_get("", "payload_json")?,
        attempts: row.try_get("", "attempts")?,
        max_attempts: row.try_get("", "max_attempts")?,
    }))
}

async fn dispatch_job(db: &DatabaseConnection, job: &OutboxJob) -> std::result::Result<(), String> {
    match job.topic.as_str() {
        OUTBOX_TOPIC_OTP_EMAIL => {
            let payload: OtpEmailJobPayload = serde_json::from_str(&job.payload_json)
                .map_err(|error| format!("invalid otp payload: {error}"))?;
            crate::controllers::migration_api::deliver_otp_email_job(&payload.to, &payload.code)
                .await;
            Ok(())
        }
        OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC => {
            let payload: MatrixProfileAvatarSyncJobPayload =
                serde_json::from_str(&job.payload_json)
                    .map_err(|error| format!("invalid matrix avatar sync payload: {error}"))?;
            let user_pid = uuid::Uuid::parse_str(&payload.user_pid)
                .map_err(|error| format!("invalid matrix avatar sync user_pid: {error}"))?;
            crate::controllers::migration_api::deliver_matrix_profile_avatar_sync_job(
                &user_pid,
                payload.profile_picture_filename.as_deref(),
            )
            .await;
            Ok(())
        }
        OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC => {
            let payload: MatrixEventMembershipSyncJobPayload =
                serde_json::from_str(&job.payload_json)
                    .map_err(|error| format!("invalid matrix membership sync payload: {error}"))?;
            let event_id = uuid::Uuid::parse_str(&payload.event_id)
                .map_err(|error| format!("invalid matrix membership sync event_id: {error}"))?;
            let profile_id = uuid::Uuid::parse_str(&payload.profile_id)
                .map_err(|error| format!("invalid matrix membership sync profile_id: {error}"))?;
            crate::controllers::migration_api::deliver_matrix_event_membership_sync_job(
                db,
                event_id,
                profile_id,
                payload.leave,
            )
            .await
        }
        OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION => {
            let payload: UploadVariantsGenerationJobPayload =
                serde_json::from_str(&job.payload_json)
                    .map_err(|error| format!("invalid upload variants job payload: {error}"))?;
            let upload_id = uuid::Uuid::parse_str(&payload.upload_id)
                .map_err(|error| format!("invalid upload variants upload_id: {error}"))?;
            crate::controllers::migration_api::deliver_upload_variants_generation_job(db, upload_id)
                .await
        }
        other => Err(format!("unsupported outbox topic: {other}")),
    }
}

async fn mark_job_done(
    db: &DatabaseConnection,
    job_id: &str,
) -> std::result::Result<(), sea_orm::DbErr> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r"
        UPDATE job_outbox
        SET processed_at = NOW(),
            locked_at = NULL,
            last_error = NULL,
            updated_at = NOW()
        WHERE id = $1::uuid
        ",
        vec![job_id.to_string().into()],
    );
    db.execute(stmt).await.map(|_| ())
}

async fn mark_job_failed(
    db: &DatabaseConnection,
    job: &OutboxJob,
    error_message: &str,
) -> std::result::Result<(), sea_orm::DbErr> {
    let clamped_error = truncate_error(error_message);

    let stmt = if job.attempts >= job.max_attempts {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r"
            UPDATE job_outbox
            SET failed_at = NOW(),
                locked_at = NULL,
                last_error = $2,
                updated_at = NOW()
            WHERE id = $1::uuid
            ",
            vec![job.id.clone().into(), clamped_error.into()],
        )
    } else {
        let backoff_secs = retry_backoff_secs(job.attempts);
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r"
            UPDATE job_outbox
            SET locked_at = NULL,
                available_at = NOW() + make_interval(secs => $2),
                last_error = $3,
                updated_at = NOW()
            WHERE id = $1::uuid
            ",
            vec![
                job.id.clone().into(),
                backoff_secs.into(),
                clamped_error.into(),
            ],
        )
    };

    tracing::warn!(
        job_id = %job.id,
        topic = %job.topic,
        attempts = job.attempts,
        max_attempts = job.max_attempts,
        error = %error_message,
        "outbox job failed"
    );
    db.execute(stmt).await.map(|_| ())
}

const fn retry_backoff_secs(attempts: i32) -> i64 {
    match attempts {
        0 | 1 => 5,
        2 => 15,
        3 => 30,
        4 => 60,
        5 => 120,
        _ => 300,
    }
}

fn truncate_error(message: &str) -> String {
    const MAX_LEN: usize = 800;
    if message.len() <= MAX_LEN {
        message.to_string()
    } else {
        message.chars().take(MAX_LEN).collect()
    }
}
