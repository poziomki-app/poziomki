//! Process-wide [`ModerationEngine`] singleton, initialized from environment
//! variables at boot (once per process, in both the API and worker).
//!
//! Contract:
//! - `MODERATION_MODEL_PATH` unset or empty → moderation is disabled; all
//!   handlers must treat a missing engine as "allow, no opinion".
//! - `MODERATION_MODEL_PATH` set, directory missing required files →
//!   **moderation disabled, boot continues** with a loud warn. This
//!   matters because `docker-compose.prod.yml` always sets the env var
//!   to a sensible default path — a misprovisioned host would otherwise
//!   crash-loop the whole API instead of just running without moderation.
//!   Operators should alert on the startup log / `moderation_engine_loaded`
//!   metric rather than relying on process death.
//! - `MODERATION_MODEL_PATH` set, files present but load fails for other
//!   reasons (corrupt ONNX, tokenizer parse error, ORT init) → fatal.
//!   That's a real bug, not a provisioning gap.
//! - `MODERATION_THREADS` (optional, default 2) caps ORT intra-op threads
//!   per inference. Keep this low; concurrency comes from `spawn_blocking`
//!   worker threads, not from ORT.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use super::{ModerationEngine, ModerationError};

static ENGINE: OnceLock<Option<Arc<ModerationEngine>>> = OnceLock::new();

const ENV_MODEL_PATH: &str = "MODERATION_MODEL_PATH";
const ENV_THREADS: &str = "MODERATION_THREADS";

/// Initialise the global engine from environment variables.
///
/// Must be called exactly once per process before any handler tries to
/// resolve the engine. Subsequent calls are a no-op.
///
/// # Errors
/// Returns [`ModerationError`] when `MODERATION_MODEL_PATH` is set but the
/// model fails to load.
pub fn init_from_env() -> Result<(), ModerationError> {
    if ENGINE.get().is_some() {
        return Ok(());
    }

    let path = std::env::var(ENV_MODEL_PATH)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    let Some(path) = path else {
        tracing::warn!(
            env = ENV_MODEL_PATH,
            "moderation disabled: env var unset or empty"
        );
        let _ = ENGINE.set(None);
        return Ok(());
    };

    let threads: usize = std::env::var(ENV_THREADS)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(2);

    let dir = PathBuf::from(&path);
    tracing::info!(path = %path, threads, "loading moderation engine");
    let started = std::time::Instant::now();
    match ModerationEngine::load_from_dir(&dir, threads) {
        Ok(engine) => {
            let elapsed_ms = started.elapsed().as_millis();
            tracing::info!(elapsed_ms, "moderation engine loaded");
            let _ = ENGINE.set(Some(Arc::new(engine)));
            Ok(())
        }
        Err(ModerationError::MissingFile(missing)) => {
            tracing::warn!(
                path = %path,
                missing = %missing.display(),
                "moderation disabled: model directory is missing required files \
                 (did the operator forget to mount MODERATION_MODEL_PATH?)"
            );
            let _ = ENGINE.set(None);
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Return the shared engine, or `None` if moderation is disabled for this
/// process. Call sites should treat `None` as "allow".
#[must_use]
pub fn shared() -> Option<Arc<ModerationEngine>> {
    ENGINE.get().and_then(Option::as_ref).map(Arc::clone)
}
