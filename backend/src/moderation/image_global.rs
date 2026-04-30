//! Process-wide [`ImageModerationEngine`] singleton, mirroring
//! [`super::global`] for the text engine. Same env-var contract:
//!
//! - `IMAGE_MODERATION_MODEL_PATH` unset → moderation disabled, all
//!   uploads pass through.
//! - `IMAGE_MODERATION_REQUIRED` truthy → strict; missing path or files
//!   is fatal at boot.
//! - `IMAGE_MODERATION_THREADS` (default 2) — ORT intra-op cap.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use super::image_engine::{ImageModerationEngine, ImageModerationError};

static ENGINE: OnceLock<Option<Arc<ImageModerationEngine>>> = OnceLock::new();

const ENV_MODEL_PATH: &str = "IMAGE_MODERATION_MODEL_PATH";
const ENV_THREADS: &str = "IMAGE_MODERATION_THREADS";
const ENV_REQUIRED: &str = "IMAGE_MODERATION_REQUIRED";

#[must_use]
pub fn is_required() -> bool {
    std::env::var(ENV_REQUIRED).is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Initialise the global image moderation engine from env. Idempotent.
///
/// # Errors
/// Returns [`ImageModerationError`] when `IMAGE_MODERATION_MODEL_PATH`
/// is set but the model fails to load (or strictly required and missing).
pub fn init_image_from_env() -> Result<(), ImageModerationError> {
    if ENGINE.get().is_some() {
        return Ok(());
    }

    let path = std::env::var(ENV_MODEL_PATH)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let strict = is_required();

    let Some(path) = path else {
        if strict {
            return Err(ImageModerationError::MissingFile(PathBuf::from(format!(
                "${ENV_MODEL_PATH} is required ({ENV_REQUIRED}={})",
                std::env::var(ENV_REQUIRED).unwrap_or_default()
            ))));
        }
        tracing::warn!(
            env = ENV_MODEL_PATH,
            "image moderation disabled: env var unset or empty"
        );
        let _ = ENGINE.set(None);
        metrics::gauge!("image_moderation_engine_loaded").set(0.0);
        return Ok(());
    };

    let threads: usize = std::env::var(ENV_THREADS)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(2);

    let dir = PathBuf::from(&path);
    tracing::info!(path = %path, threads, "loading image moderation engine");
    let started = std::time::Instant::now();
    match ImageModerationEngine::load_from_dir(&dir, threads) {
        Ok(engine) => {
            let elapsed_ms = started.elapsed().as_millis();
            tracing::info!(elapsed_ms, "image moderation engine loaded");
            let _ = ENGINE.set(Some(Arc::new(engine)));
            metrics::gauge!("image_moderation_engine_loaded").set(1.0);
            Ok(())
        }
        Err(ImageModerationError::MissingFile(missing)) => {
            if strict {
                return Err(ImageModerationError::MissingFile(missing));
            }
            tracing::warn!(
                path = %path,
                missing = %missing.display(),
                "image moderation disabled: model directory missing required files"
            );
            let _ = ENGINE.set(None);
            metrics::gauge!("image_moderation_engine_loaded").set(0.0);
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Return the shared image engine, or `None` if disabled. Treat `None`
/// as "allow" at all call sites.
#[must_use]
pub fn shared_image() -> Option<Arc<ImageModerationEngine>> {
    ENGINE.get().and_then(Option::as_ref).map(Arc::clone)
}
