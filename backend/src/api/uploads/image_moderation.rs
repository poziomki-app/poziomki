//! Synchronous NSFW gate for user-uploaded images. Runs after the cheap
//! validation chain (mime/size/magic/dimensions/strip) so we never spend
//! inference cycles on already-rejected input.
//!
//! When the engine is disabled (env unset, dev/CI), uploads are allowed
//! through unmoderated — that's intentional. When the engine is supposed
//! to run (`IMAGE_MODERATION_REQUIRED=true`) but the inference call errors,
//! the upload is rejected with 503 so a transient model outage cannot be
//! used to slip NSFW content past the gate.

use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use super::uploads_multipart::HandlerError;
use crate::api::error_response;
use crate::api::ErrorSpec;
use crate::moderation::{image_moderation_required, shared_image, ImageVerdict};

fn moderation_unavailable_response(headers: &HeaderMap) -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        headers,
        ErrorSpec {
            error: "Moderacja zdjęć jest tymczasowo niedostępna. Spróbuj ponownie za chwilę."
                .to_string(),
            code: "IMAGE_MODERATION_UNAVAILABLE",
            details: None,
        },
    )
}

/// Run NSFW moderation against `bytes`. Bytes should already be the
/// sanitized, re-encoded payload (smaller, predictable format) — both
/// upload paths call this after `strip_image_metadata`.
///
/// Returns `Ok(None)` when the upload may proceed, `Ok(Some(resp))` with
/// a 422 (rejected content) or 503 (engine unavailable in strict mode).
/// In strict mode (`IMAGE_MODERATION_REQUIRED=true`), inference errors
/// fail closed — a model hiccup must not become a way to bypass NSFW
/// checks. Outside strict mode, errors are logged and the upload is let
/// through (dev/CI behaviour).
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

    let strict = image_moderation_required();
    let score = match result {
        Ok(Ok(s)) => s,
        Ok(Err(err)) => {
            tracing::error!(%err, elapsed_ms, strict, "image moderation inference failed");
            metrics::counter!("image_moderation_errors_total", "kind" => "inference").increment(1);
            return if strict {
                Ok(Some(moderation_unavailable_response(headers)))
            } else {
                Ok(None)
            };
        }
        Err(err) => {
            tracing::error!(%err, elapsed_ms, strict, "image moderation task join failed");
            metrics::counter!("image_moderation_errors_total", "kind" => "join").increment(1);
            return if strict {
                Ok(Some(moderation_unavailable_response(headers)))
            } else {
                Ok(None)
            };
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
