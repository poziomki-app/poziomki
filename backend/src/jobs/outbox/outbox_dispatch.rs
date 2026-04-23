use diesel::sql_types::{BigInt, Text};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use super::OutboxJob;

pub(super) const OUTBOX_TOPIC_OTP_EMAIL: &str = "otp_email_send";
pub(super) const OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION: &str = "upload_variants_generation";
pub(super) const OUTBOX_TOPIC_CHAT_MEMBERSHIP_SYNC: &str = "chat_membership_sync";
pub(super) const OUTBOX_TOPIC_MODERATION_SCAN: &str = "moderation_scan";

#[derive(Debug, Deserialize)]
struct OtpEmailJobPayload {
    to: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct UploadVariantsGenerationJobPayload {
    upload_id: String,
}

#[derive(Debug, Deserialize)]
struct ChatMembershipSyncJobPayload {
    event_id: String,
    profile_id: String,
    leave: bool,
}

#[derive(Debug, Deserialize)]
struct ModerationScanJobPayload {
    // `bio` or `message` — which surface produced this scan request.
    // Selects the lookup table and the threshold preset; log/metric
    // labelling too. Accept unknown values rather than rejecting the job.
    target_kind: String,
    target_id: String,
    // NB: the target's text is intentionally NOT carried in the payload.
    // Dispatch re-reads the current body from the source table so that
    // (a) a soft-delete before scan time cancels the scan, (b) edits are
    // always scanned on the latest content, and (c) user text is not
    // duplicated into `job_outbox` indefinitely.
}

#[tracing::instrument(skip(job), fields(job_id = %job.id, job_topic = %job.topic))]
pub(super) async fn dispatch_job(job: &OutboxJob) -> std::result::Result<(), String> {
    match job.topic.as_str() {
        OUTBOX_TOPIC_OTP_EMAIL => dispatch_otp_email(&job.payload_json).await,
        OUTBOX_TOPIC_UPLOAD_VARIANTS_GENERATION => {
            dispatch_upload_variants(&job.payload_json).await
        }
        OUTBOX_TOPIC_CHAT_MEMBERSHIP_SYNC => dispatch_chat_membership_sync(&job.payload_json).await,
        OUTBOX_TOPIC_MODERATION_SCAN => dispatch_moderation_scan(&job.payload_json).await,
        other => Err(format!("unsupported outbox topic: {other}")),
    }
}

async fn dispatch_otp_email(payload_json: &str) -> std::result::Result<(), String> {
    let payload: OtpEmailJobPayload =
        serde_json::from_str(payload_json).map_err(|e| format!("invalid otp payload: {e}"))?;
    crate::api::deliver_otp_email_job(&payload.to, &payload.code).await
}

async fn dispatch_chat_membership_sync(payload_json: &str) -> std::result::Result<(), String> {
    let payload: ChatMembershipSyncJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid chat membership sync payload: {e}"))?;
    let event_id = uuid::Uuid::parse_str(&payload.event_id)
        .map_err(|e| format!("invalid chat membership sync event_id: {e}"))?;
    let profile_id = uuid::Uuid::parse_str(&payload.profile_id)
        .map_err(|e| format!("invalid chat membership sync profile_id: {e}"))?;
    crate::api::deliver_chat_membership_sync_job(event_id, profile_id, payload.leave).await
}

async fn dispatch_moderation_scan(payload_json: &str) -> std::result::Result<(), String> {
    let payload: ModerationScanJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid moderation scan payload: {e}"))?;

    let Some(engine) = crate::moderation::shared() else {
        // Operator didn't wire a model in this process — drop the job
        // silently. Worth logging once at boot (`global.rs` already does
        // that), so here we just no-op to avoid log spam.
        metrics::counter!("moderation_scan_skipped_total", "reason" => "engine_disabled")
            .increment(1);
        return Ok(());
    };

    // Re-read the current text from its source table. If the row is gone
    // (user deleted / soft-deleted between enqueue and dispatch) or the
    // job's target_kind is unknown, skip — don't treat as a dispatch
    // failure that would retry forever.
    let text = match resolve_moderation_text(&payload).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            metrics::counter!(
                "moderation_scan_skipped_total",
                "reason" => "target_missing"
            )
            .increment(1);
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    if text.trim().is_empty() {
        return Ok(());
    }

    let started = std::time::Instant::now();
    // ONNX inference is CPU-bound and blocking; run it off the async
    // runtime. `spawn_blocking` propagates panics as `JoinError`.
    let scores = tokio::task::spawn_blocking(move || engine.score(&text))
        .await
        .map_err(|e| format!("moderation inference panicked: {e}"))?
        .map_err(|e| format!("moderation inference failed: {e}"))?;

    let thresholds = if payload.target_kind == "bio" {
        crate::moderation::Thresholds::BIO
    } else {
        crate::moderation::Thresholds::CHAT
    };
    let verdict = scores.verdict(&thresholds);
    let flagged = scores.flagged(&thresholds);

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    metrics::histogram!("moderation_inference_latency_ms", "kind" => payload.target_kind.clone())
        .record(elapsed_ms);
    metrics::counter!(
        "moderation_verdicts_total",
        "kind" => payload.target_kind.clone(),
        "verdict" => format!("{verdict:?}")
    )
    .increment(1);

    if matches!(
        verdict,
        crate::moderation::Verdict::Flag | crate::moderation::Verdict::Block
    ) {
        let flagged_labels: Vec<String> = flagged
            .iter()
            .map(|(c, s)| format!("{}={:.2}", c.as_str(), s))
            .collect();
        tracing::warn!(
            target_kind = %payload.target_kind,
            target_id = %payload.target_id,
            verdict = ?verdict,
            self_harm = scores.self_harm,
            hate = scores.hate,
            vulgar = scores.vulgar,
            sex = scores.sex,
            crime = scores.crime,
            flagged = ?flagged_labels,
            elapsed_ms,
            "moderation: content flagged"
        );
    } else {
        tracing::debug!(
            target_kind = %payload.target_kind,
            target_id = %payload.target_id,
            elapsed_ms,
            "moderation: content allowed"
        );
    }

    Ok(())
}

/// Look up the current text behind a moderation scan job.
///
/// Returns `Ok(None)` for a legitimately missing target (row gone,
/// soft-deleted, or unknown `target_kind`) — the caller should treat that
/// as a skipped job, not a retryable error. Returns `Err` only for real
/// infrastructure failures (DB pool / connection errors).
async fn resolve_moderation_text(
    payload: &ModerationScanJobPayload,
) -> std::result::Result<Option<String>, String> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let target_id = uuid::Uuid::parse_str(&payload.target_id)
        .map_err(|e| format!("invalid moderation scan target_id: {e}"))?;

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| format!("moderation scan db conn: {e}"))?;

    match payload.target_kind.as_str() {
        "message" => {
            use crate::db::schema::messages;
            let row: Option<(String, Option<chrono::DateTime<chrono::Utc>>)> = messages::table
                .filter(messages::id.eq(target_id))
                .select((messages::body, messages::deleted_at))
                .first(&mut conn)
                .await
                .optional()
                .map_err(|e| format!("moderation scan fetch message: {e}"))?;
            Ok(row.and_then(|(body, deleted_at)| {
                if deleted_at.is_some() {
                    None
                } else {
                    Some(body)
                }
            }))
        }
        "bio" => {
            use crate::db::schema::profiles;
            let bio: Option<Option<String>> = profiles::table
                .filter(profiles::id.eq(target_id))
                .select(profiles::bio)
                .first(&mut conn)
                .await
                .optional()
                .map_err(|e| format!("moderation scan fetch bio: {e}"))?;
            Ok(bio.flatten())
        }
        other => {
            tracing::warn!(
                target_kind = %other,
                "moderation scan: unknown target_kind, skipping"
            );
            Ok(None)
        }
    }
}

async fn dispatch_upload_variants(payload_json: &str) -> std::result::Result<(), String> {
    let payload: UploadVariantsGenerationJobPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid upload variants job payload: {e}"))?;
    let upload_id = uuid::Uuid::parse_str(&payload.upload_id)
        .map_err(|e| format!("invalid upload variants upload_id: {e}"))?;
    crate::api::deliver_upload_variants_generation_job(upload_id).await
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
