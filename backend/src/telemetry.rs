use crate::error::{AppError, AppResult};
use crate::jobs::OutboxStatsSnapshot;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::sync::OnceLock;
use std::time::Duration;

const API_METRICS_PORT: u16 = 9092;
const WORKER_METRICS_PORT: u16 = 9093;

#[derive(Clone, Copy, Debug)]
pub enum ProcessKind {
    Api,
    Worker,
}

impl ProcessKind {
    const fn default_metrics_port(self) -> u16 {
        match self {
            Self::Api => API_METRICS_PORT,
            Self::Worker => WORKER_METRICS_PORT,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Worker => "worker",
        }
    }
}

pub fn init_metrics_exporter(kind: ProcessKind) -> AppResult<()> {
    static METRICS_INIT: OnceLock<()> = OnceLock::new();
    if METRICS_INIT.get().is_some() {
        return Ok(());
    }

    describe_metrics();

    let port = metrics_port(kind)?;
    PrometheusBuilder::new()
        .add_global_label("service", kind.as_str())
        .with_http_listener(([127, 0, 0, 1], port))
        .install()
        .map_err(|error| AppError::message(format!("metrics init: {error}")))?;

    let _ = METRICS_INIT.set(());
    tracing::info!(
        port,
        service = kind.as_str(),
        "Prometheus metrics exporter started"
    );
    Ok(())
}

pub fn record_http_request(method: &str, route: &str, status: u16, duration: Duration) {
    let status_label = status.to_string();
    let status_class = status_class(status);
    metrics::counter!(
        "poziomki_http_requests_total",
        "method" => method.to_string(),
        "route" => route.to_string(),
        "status" => status_label,
        "status_class" => status_class.to_string()
    )
    .increment(1);

    metrics::histogram!(
        "poziomki_http_request_duration_seconds",
        "method" => method.to_string(),
        "route" => route.to_string(),
        "status_class" => status_class.to_string()
    )
    .record(duration.as_secs_f64());
}

pub fn record_outbox_job_result(topic: &str, result: &'static str) {
    metrics::counter!(
        "poziomki_outbox_jobs_total",
        "topic" => topic.to_string(),
        "result" => result.to_string()
    )
    .increment(1);
}

pub fn update_outbox_metrics(snapshot: &OutboxStatsSnapshot) {
    metrics::gauge!("poziomki_outbox_pending_jobs").set(i64_as_f64(snapshot.pending_jobs));
    metrics::gauge!("poziomki_outbox_ready_jobs").set(i64_as_f64(snapshot.ready_jobs));
    metrics::gauge!("poziomki_outbox_retrying_jobs").set(i64_as_f64(snapshot.retrying_jobs));
    metrics::gauge!("poziomki_outbox_inflight_jobs").set(i64_as_f64(snapshot.inflight_jobs));
    metrics::gauge!("poziomki_outbox_failed_jobs").set(i64_as_f64(snapshot.failed_jobs));
    metrics::gauge!("poziomki_outbox_exhausted_jobs").set(i64_as_f64(snapshot.exhausted_jobs));
    metrics::gauge!("poziomki_outbox_processed_jobs_24h")
        .set(i64_as_f64(snapshot.processed_jobs_24h));
    metrics::gauge!("poziomki_outbox_oldest_ready_job_age_seconds")
        .set(i64_as_f64(snapshot.oldest_ready_job_age_seconds));
    metrics::gauge!("poziomki_outbox_oldest_pending_job_age_seconds")
        .set(i64_as_f64(snapshot.oldest_pending_job_age_seconds));

    let worker_degraded = snapshot.failed_jobs > 0 || snapshot.oldest_ready_job_age_seconds > 60;
    metrics::gauge!("poziomki_outbox_worker_degraded").set(if worker_degraded { 1.0 } else { 0.0 });
}

#[must_use]
pub fn metrics_route_label(matched_path: Option<&str>) -> &str {
    matched_path.unwrap_or("unmatched")
}

fn metrics_port(kind: ProcessKind) -> AppResult<u16> {
    match std::env::var("METRICS_PORT") {
        Ok(raw) if !raw.trim().is_empty() => raw
            .parse::<u16>()
            .map_err(|error| AppError::message(format!("invalid METRICS_PORT: {error}"))),
        _ => Ok(kind.default_metrics_port()),
    }
}

const fn status_class(status: u16) -> &'static str {
    match status {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "unknown",
    }
}

#[allow(clippy::cast_precision_loss)]
const fn i64_as_f64(value: i64) -> f64 {
    value as f64
}

fn describe_metrics() {
    metrics::describe_counter!(
        "poziomki_http_requests_total",
        "Total HTTP requests handled by the API, partitioned by method, route, and status."
    );
    metrics::describe_histogram!(
        "poziomki_http_request_duration_seconds",
        "HTTP request latency in seconds, partitioned by method, route, and status class."
    );
    metrics::describe_counter!(
        "poziomki_outbox_jobs_total",
        "Total outbox job attempts by topic and outcome."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_pending_jobs",
        "Number of queued outbox jobs that have not been processed yet."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_ready_jobs",
        "Number of outbox jobs ready to be claimed immediately."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_retrying_jobs",
        "Number of outbox jobs that have failed before and are waiting to retry."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_inflight_jobs",
        "Number of outbox jobs currently locked by a worker."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_failed_jobs",
        "Number of outbox jobs marked as failed."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_exhausted_jobs",
        "Number of outbox jobs that exhausted all retry attempts."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_processed_jobs_24h",
        "Number of outbox jobs processed during the last 24 hours."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_oldest_ready_job_age_seconds",
        "Age of the oldest ready outbox job in seconds."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_oldest_pending_job_age_seconds",
        "Age of the oldest pending outbox job in seconds."
    );
    metrics::describe_gauge!(
        "poziomki_outbox_worker_degraded",
        "Whether the outbox worker is currently degraded according to the queue snapshot."
    );
}

#[cfg(test)]
mod tests {
    use super::{
        metrics_route_label, status_class, ProcessKind, API_METRICS_PORT, WORKER_METRICS_PORT,
    };

    #[test]
    fn uses_default_route_label_for_unmatched_requests() {
        assert_eq!(metrics_route_label(None), "unmatched");
        assert_eq!(
            metrics_route_label(Some("/api/v1/events/{id}")),
            "/api/v1/events/{id}"
        );
    }

    #[test]
    fn maps_status_to_status_class() {
        assert_eq!(status_class(200), "2xx");
        assert_eq!(status_class(404), "4xx");
        assert_eq!(status_class(503), "5xx");
    }

    #[test]
    fn uses_stable_default_ports_per_process() {
        assert_eq!(ProcessKind::Api.default_metrics_port(), API_METRICS_PORT);
        assert_eq!(
            ProcessKind::Worker.default_metrics_port(),
            WORKER_METRICS_PORT
        );
    }
}
