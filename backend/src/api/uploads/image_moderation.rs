//! Synchronous NSFW gate for user-uploaded images. Runs after the cheap
//! validation chain (mime/size/magic/dimensions/strip) so we never spend
//! inference cycles on already-rejected input.
//!
//! Behaviour mirrors the bio moderation gate in
//! `api/profiles/write_handler.rs`: returns `Ok(None)` when the engine is
//! disabled, the image is allowed, or moderation is unreachable for an
//! infrastructural reason (we fail open on infra errors but log loudly —
//! the alternative is dropping every upload during an ORT hiccup).

use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use super::uploads_multipart::HandlerError;
use crate::api::error_response;
use crate::api::ErrorSpec;
use crate::moderation::{shared_image, ImageVerdict};

/// Run NSFW moderation against `bytes`. Bytes should already be the
/// sanitized, re-encoded payload (smaller, predictable format) — both
/// upload paths call this after `strip_image_metadata`.
///
/// Returns `Ok(None)` when the upload may proceed, or `Ok(Some(resp))`
/// with a 422 the caller must surface verbatim. Inference errors are
/// logged and treated as "allow"; we don't want a model hiccup to cause
/// a global upload outage.
pub(super) async fn moderate_upload_image(
    headers: &HeaderMap,
    bytes: &[u8],
) -> std::result::Result<Option<Response>, HandlerError> {
    let Some(engine) = shared_image() else {
        return Ok(None);
    };

    let owned = bytes.to_vec();
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || engine.score(&owned)).await;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    metrics::histogram!("image_moderation_inference_latency_ms").record(elapsed_ms);

    let score = match result {
        Ok(Ok(s)) => s,
        Ok(Err(err)) => {
            tracing::error!(%err, elapsed_ms, "image moderation inference failed; allowing upload");
            metrics::counter!("image_moderation_errors_total", "kind" => "inference").increment(1);
            return Ok(None);
        }
        Err(err) => {
            tracing::error!(%err, elapsed_ms, "image moderation task join failed; allowing upload");
            metrics::counter!("image_moderation_errors_total", "kind" => "join").increment(1);
            return Ok(None);
        }
    };

    let verdict = score.verdict();
    metrics::counter!(
        "image_moderation_verdicts_total",
        "verdict" => verdict.as_str()
    )
    .increment(1);

    match verdict {
        ImageVerdict::Allow => Ok(None),
        ImageVerdict::Block => {
            tracing::warn!(
                nsfw = score.nsfw,
                elapsed_ms,
                "image moderation: blocked on upload"
            );
            Ok(Some(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                headers,
                ErrorSpec {
                    error: "Przesłany obraz narusza zasady społeczności i nie może zostać \
                            zapisany. Wybierz inne zdjęcie."
                        .to_string(),
                    code: "IMAGE_CONTENT_REJECTED",
                    details: Some(serde_json::json!({ "nsfw": score.nsfw })),
                },
            )))
        }
    }
}

/// Convenience: wrap the rejection-or-pass tuple into the existing
/// `bad_request`-shaped error type used by both upload paths so call
/// sites stay symmetric with the rest of the validation chain.
pub(super) async fn moderate_upload_image_or_reject(
    headers: &HeaderMap,
    bytes: &[u8],
) -> std::result::Result<(), HandlerError> {
    moderate_upload_image(headers, bytes)
        .await?
        .map_or_else(|| Ok(()), |resp| Err(Box::new(resp)))
}
