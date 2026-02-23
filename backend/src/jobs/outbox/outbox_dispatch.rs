use diesel::sql_types::{BigInt, Text};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use super::OutboxJob;

pub(super) const OUTBOX_TOPIC_OTP_EMAIL: &str = "otp_email_send";
pub(super) const OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC: &str = "matrix_profile_avatar_sync";
pub(super) const OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC: &str = "matrix_event_membership_sync";
pub(super) const OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION: &str = "upload_variants_generation";

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

pub(super) async fn dispatch_job(job: &OutboxJob) -> std::result::Result<(), String> {
    match job.topic.as_str() {
        OUTBOX_TOPIC_OTP_EMAIL => dispatch_otp_email(&job.payload_json).await,
        OUTBOX_TOPIC_MATRIX_PROFILE_AVATAR_SYNC => {
            dispatch_matrix_avatar_sync(&job.payload_json).await
        }
        OUTBOX_TOPIC_MATRIX_EVENT_MEMBERSHIP_SYNC => {
            dispatch_matrix_membership_sync(&job.payload_json).await
        }
        OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION => {
            dispatch_upload_variants(&job.payload_json).await
        }
        other => Err(format!("unsupported outbox topic: {other}")),
    }
}

async fn dispatch_otp_email(payload_json: &str) -> std::result::Result<(), String> {
    let payload: OtpEmailJobPayload =
        serde_json::from_str(payload_json).map_err(|e| format!("invalid otp payload: {e}"))?;
    crate::controllers::api::deliver_otp_email_job(&payload.to, &payload.code).await;
    Ok(())
}

async fn dispatch_matrix_avatar_sync(payload_json: &str) -> std::result::Result<(), String> {
    let payload: MatrixProfileAvatarSyncJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid matrix avatar sync payload: {e}"))?;
    let user_pid = uuid::Uuid::parse_str(&payload.user_pid)
        .map_err(|e| format!("invalid matrix avatar sync user_pid: {e}"))?;
    crate::controllers::api::deliver_matrix_profile_avatar_sync_job(
        &user_pid,
        payload.profile_picture_filename.as_deref(),
    )
    .await;
    Ok(())
}

async fn dispatch_matrix_membership_sync(payload_json: &str) -> std::result::Result<(), String> {
    let payload: MatrixEventMembershipSyncJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid matrix membership sync payload: {e}"))?;
    let event_id = uuid::Uuid::parse_str(&payload.event_id)
        .map_err(|e| format!("invalid matrix membership sync event_id: {e}"))?;
    let profile_id = uuid::Uuid::parse_str(&payload.profile_id)
        .map_err(|e| format!("invalid matrix membership sync profile_id: {e}"))?;
    crate::controllers::api::deliver_matrix_event_membership_sync_job(
        event_id,
        profile_id,
        payload.leave,
    )
    .await
}

async fn dispatch_upload_variants(payload_json: &str) -> std::result::Result<(), String> {
    let payload: UploadVariantsGenerationJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid upload variants job payload: {e}"))?;
    let upload_id = uuid::Uuid::parse_str(&payload.upload_id)
        .map_err(|e| format!("invalid upload variants upload_id: {e}"))?;
    crate::controllers::api::deliver_upload_variants_generation_job(upload_id).await
}

pub(super) async fn mark_job_done(job_id: &str) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    diesel::sql_query(
        r"
        UPDATE job_outbox
        SET processed_at = NOW(),
            locked_at = NULL,
            last_error = NULL,
            updated_at = NOW()
        WHERE id = $1::uuid
        ",
    )
    .bind::<Text, _>(job_id)
    .execute(&mut conn)
    .await?;
    Ok(())
}

pub(super) async fn mark_job_failed(
    job: &OutboxJob,
    error_message: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let clamped_error = truncate_error(error_message);

    let mut conn = crate::db::conn().await?;

    if job.attempts >= job.max_attempts {
        diesel::sql_query(
            r"
            UPDATE job_outbox
            SET failed_at = NOW(),
                locked_at = NULL,
                last_error = $2,
                updated_at = NOW()
            WHERE id = $1::uuid
            ",
        )
        .bind::<Text, _>(&job.id)
        .bind::<Text, _>(&clamped_error)
        .execute(&mut conn)
        .await?;
    } else {
        let backoff_secs = retry_backoff_secs(job.attempts);
        diesel::sql_query(
            r"
            UPDATE job_outbox
            SET locked_at = NULL,
                available_at = NOW() + make_interval(secs => $2),
                last_error = $3,
                updated_at = NOW()
            WHERE id = $1::uuid
            ",
        )
        .bind::<Text, _>(&job.id)
        .bind::<BigInt, _>(backoff_secs)
        .bind::<Text, _>(&clamped_error)
        .execute(&mut conn)
        .await?;
    }

    tracing::warn!(
        job_id = %job.id,
        topic = %job.topic,
        attempts = job.attempts,
        max_attempts = job.max_attempts,
        error = %error_message,
        "outbox job failed"
    );
    Ok(())
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
