mod api;
pub mod collector;
mod middleware;
mod sampler;
pub mod store;

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub use collector::{MetricsConfig, MetricsStore};
pub use middleware::metrics_middleware;

static METRICS: OnceLock<MetricsStore> = OnceLock::new();

/// Returns a reference to the global metrics store.
/// Returns `None` if `init()` has not been called yet.
pub fn metrics() -> Option<&'static MetricsStore> {
    METRICS.get()
}

/// Initialize the metrics system.
///
/// Starts the background sampler task and stores the global `MetricsStore`.
/// Idempotent: subsequent calls are no-ops.
pub fn init(config: MetricsConfig) {
    if METRICS.get().is_some() {
        return;
    }
    let store = MetricsStore::new(config);
    if METRICS.set(store).is_err() {
        return;
    }
    if let Some(m) = METRICS.get() {
        sampler::spawn_sampler(m);
    }
}

/// Returns an Axum `Router<()>` with the metrics endpoints mounted.
pub fn routes() -> axum::Router {
    api::routes()
}

/// Current time as a Unix epoch in seconds (u32).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn now_epoch() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}
