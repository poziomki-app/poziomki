// Metrics counters/gauges are small values; precision loss from u64→f32 / i64→f32 is acceptable.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod api;
pub mod collector;
mod middleware;
mod sampler;
pub mod store;

use std::sync::OnceLock;

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
