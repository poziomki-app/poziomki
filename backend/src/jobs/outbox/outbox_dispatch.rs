use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::db::schema::messages::dsl as m;

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
    // Log/metric labelling only.
    target_kind: String,
    target_id: String,
    // The body snapshotted at the moment the scan was enqueued. We scan
    // this, NOT the current row, so a sender can't bypass moderation by
    // editing or deleting their message between broadcast and scan.
    // Processed rows are purged by the cleanup loop after
    // `OUTBOX_PROCESSED_RETENTION_DAYS` (see jobs/cleanup.rs).
    body: String,
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

    // Scan the body snapshot carried in the payload, NOT the current row.
    // This is what recipients actually saw: editing or deleting the
    // message after broadcast cannot evade the scan.
    if payload.body.trim().is_empty() {
        return Ok(());
    }
    // Move the body into the blocking closure — payload is owned here, so
    // no allocation beyond the deserialise that already happened.
    let ModerationScanJobPayload {
        target_kind,
        target_id,
        body,
    } = payload;

    let started = std::time::Instant::now();
    // ONNX inference is CPU-bound and blocking; run it off the async
    // runtime. `spawn_blocking` propagates panics as `JoinError`.
    let scores = tokio::task::spawn_blocking(move || engine.score(&body))
        .await
        .map_err(|e| format!("moderation inference panicked: {e}"))?
        .map_err(|e| format!("moderation inference failed: {e}"))?;

    // Only chat messages go through the outbox path today; bios are
    // moderated synchronously on publish with their own stricter preset.
    let thresholds = crate::moderation::Thresholds::CHAT;
    let verdict = scores.verdict(&thresholds);
    let flagged = scores.flagged(&thresholds);

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    metrics::histogram!("moderation_inference_latency_ms", "kind" => target_kind.clone())
        .record(elapsed_ms);
    metrics::counter!(
        "moderation_verdicts_total",
        "kind" => target_kind.clone(),
        "verdict" => verdict.as_str()
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
            target_kind = %target_kind,
            target_id = %target_id,
            verdict = verdict.as_str(),
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
            target_kind = %target_kind,
            target_id = %target_id,
            elapsed_ms,
            "moderation: content allowed"
        );
    }

    // Persist the verdict on the message row so clients can render a
    // blur overlay on next history fetch / conversation reload. Worker
    // connects with BYPASSRLS, so a direct UPDATE bypasses the chat
    // membership policies that gate normal callers.
    //
    // Live WS broadcast of the verdict change is intentionally not
    // wired in this PR — would require LISTEN/NOTIFY plumbing between
    // worker and api processes. Eventual consistency on next read is
    // acceptable for v1 (typical user opens the chat and sees the
    // blur immediately on history load).
    if target_kind == "message" {
        let message_uuid = match uuid::Uuid::parse_str(&target_id) {
            Ok(u) => u,
            Err(e) => return Err(format!("invalid message target_id {target_id}: {e}")),
        };
        let verdict_str = verdict.as_str().to_string();
        let categories: Vec<String> = flagged
            .iter()
            .map(|(c, _)| c.as_str().to_string())
            .collect();
        let mut conn = crate::db::conn()
            .await
            .map_err(|e| format!("moderation write-back: pool: {e}"))?;
        let now = chrono::Utc::now();
        diesel::update(m::messages.filter(m::id.eq(message_uuid)))
            .set((
                m::moderation_verdict.eq(verdict_str),
                m::moderation_categories.eq(categories),
                m::moderation_scanned_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| format!("moderation write-back: update: {e}"))?;
    }

    Ok(())
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
