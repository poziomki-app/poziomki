use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::store::TimeSeries;

/// Number of latency histogram buckets.
const HISTOGRAM_BUCKETS: usize = 12;

/// Bucket upper bounds in microseconds.
/// <1ms, <2ms, <5ms, <10ms, <25ms, <50ms, <100ms, <250ms, <500ms, <1s, <2s, <5s
const BUCKET_UPPER_MICROS: [u64; HISTOGRAM_BUCKETS] = [
    1_000, 2_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000, 2_000_000,
    5_000_000,
];

/// A latency histogram using atomic counters.
///
/// Records durations into buckets, then `drain_p95` computes the approximate
/// p95 latency and resets the counters.
pub struct LatencyHistogram {
    buckets: [AtomicU64; HISTOGRAM_BUCKETS],
    overflow: AtomicU64,
    count: AtomicU64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            overflow: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Record a duration sample.
    pub fn record(&self, duration: std::time::Duration) {
        let micros = duration.as_micros().min(u128::from(u64::MAX)) as u64;
        self.count.fetch_add(1, Ordering::Relaxed);

        let mut placed = false;
        for (i, &upper) in BUCKET_UPPER_MICROS.iter().enumerate() {
            if micros < upper {
                if let Some(bucket) = self.buckets.get(i) {
                    bucket.fetch_add(1, Ordering::Relaxed);
                }
                placed = true;
                break;
            }
        }
        if !placed {
            self.overflow.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Drain the histogram, compute approximate p95 latency in milliseconds, and reset.
    pub fn drain_p95(&self) -> f32 {
        let total = self.count.swap(0, Ordering::Relaxed);

        if total == 0 {
            return 0.0;
        }

        let target = (total as f64 * 0.95).ceil() as u64;
        let mut cumulative: u64 = 0;
        let mut p95_ms = 5000.0;

        for (i, bucket) in self.buckets.iter().enumerate() {
            let bucket_count = bucket.swap(0, Ordering::Relaxed);
            cumulative += bucket_count;
            if cumulative >= target {
                if let Some(&upper) = BUCKET_UPPER_MICROS.get(i) {
                    p95_ms = upper as f32 / 1000.0;
                }
                break;
            }
        }
        let _overflow_count = self.overflow.swap(0, Ordering::Relaxed);
        p95_ms
    }
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

/// A pair of atomic counters for tracking success/failure.
pub struct CounterPair {
    pub success: AtomicU64,
    pub failure: AtomicU64,
}

impl CounterPair {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            success: AtomicU64::new(0),
            failure: AtomicU64::new(0),
        }
    }

    /// Drain and return `(success_count, failure_count)`.
    pub fn drain(&self) -> (u64, u64) {
        let s = self.success.swap(0, Ordering::Relaxed);
        let f = self.failure.swap(0, Ordering::Relaxed);
        (s, f)
    }

    pub fn inc_success(&self) {
        self.success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_failure(&self) {
        self.failure.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for CounterPair {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OutboxMetricsSnapshot {
    pub pending_jobs: i64,
    pub ready_jobs: i64,
    pub retrying_jobs: i64,
    pub inflight_jobs: i64,
    pub failed_jobs: i64,
    pub oldest_ready_job_age_seconds: i64,
    pub oldest_pending_job_age_seconds: i64,
}

/// Future type for the outbox snapshot callback.
type OutboxSnapshotFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Option<OutboxMetricsSnapshot>> + Send>>;

/// Callback configuration for sampling external state.
///
/// These closures decouple the metrics module from other backend internals.
pub struct MetricsConfig {
    /// Returns `(pool_size, available, waiting)` or `None`.
    pub pool_status: Box<dyn Fn() -> Option<(u32, u32, u32)> + Send + Sync>,

    /// Returns the number of entries in the auth cache.
    pub auth_cache_size: Box<dyn Fn() -> usize + Send + Sync>,

    /// Returns outbox queue depths and ages or `None`.
    pub outbox_snapshot: Box<dyn Fn() -> OutboxSnapshotFuture + Send + Sync>,
}

/// Endpoint group for latency tracking.
#[derive(Clone, Copy)]
pub enum EndpointGroup {
    Auth,
    Profiles,
    Events,
    Uploads,
    Search,
    Matching,
    Matrix,
    Other,
}

impl EndpointGroup {
    pub fn from_path(path: &str) -> Self {
        if path.starts_with("/api/v1/auth") {
            Self::Auth
        } else if path.starts_with("/api/v1/profiles") {
            Self::Profiles
        } else if path.starts_with("/api/v1/events") {
            Self::Events
        } else if path.starts_with("/api/v1/uploads") {
            Self::Uploads
        } else if path.starts_with("/api/v1/search") || path.starts_with("/api/v1/messages/search")
        {
            Self::Search
        } else if path.starts_with("/api/v1/matching") {
            Self::Matching
        } else if path.starts_with("/api/v1/matrix") || path.starts_with("/_matrix") {
            Self::Matrix
        } else {
            Self::Other
        }
    }
}

/// The central metrics store holding all time series, histograms, and counters.
pub struct MetricsStore {
    // -- Chart 1: Request rate + errors --
    pub req_total: AtomicU64,
    pub req_4xx: AtomicU64,
    pub req_5xx: AtomicU64,
    pub ts_req_rate: TimeSeries,
    pub ts_4xx_rate: TimeSeries,
    pub ts_5xx_rate: TimeSeries,

    // -- Chart 2: HTTP concurrency --
    pub http_inflight: AtomicU64,
    pub ts_http_inflight: TimeSeries,

    // -- Chart 3: p95 latency per endpoint group --
    pub latency_auth: LatencyHistogram,
    pub latency_profiles: LatencyHistogram,
    pub latency_events: LatencyHistogram,
    pub latency_uploads: LatencyHistogram,
    pub latency_search: LatencyHistogram,
    pub latency_matching: LatencyHistogram,
    pub latency_matrix: LatencyHistogram,
    pub ts_p95_auth: TimeSeries,
    pub ts_p95_profiles: TimeSeries,
    pub ts_p95_events: TimeSeries,
    pub ts_p95_uploads: TimeSeries,
    pub ts_p95_search: TimeSeries,
    pub ts_p95_matching: TimeSeries,
    pub ts_p95_matrix: TimeSeries,

    // -- Chart 4: DB pool utilization --
    pub ts_pool_size: TimeSeries,
    pub ts_pool_available: TimeSeries,
    pub ts_pool_waiting: TimeSeries,

    // -- Chart 5: DB query duration p95 --
    pub latency_db_conn: LatencyHistogram,
    pub ts_p95_db_conn: TimeSeries,

    // -- Chart 6: Outbox queue depth --
    pub ts_outbox_pending: TimeSeries,
    pub ts_outbox_ready: TimeSeries,
    pub ts_outbox_retrying: TimeSeries,
    pub ts_outbox_inflight: TimeSeries,
    pub ts_outbox_failed: TimeSeries,

    // -- Chart 7: Outbox lag --
    pub ts_outbox_oldest_ready_age: TimeSeries,
    pub ts_outbox_oldest_pending_age: TimeSeries,

    // -- Chart 8: Auth cache hit rate --
    pub auth_cache_hits: AtomicU64,
    pub auth_cache_misses: AtomicU64,
    pub ts_auth_hit_rate: TimeSeries,
    pub ts_auth_cache_entries: TimeSeries,

    // -- Chart 9: Matrix deliverability --
    pub matrix_room_create: CounterPair,
    pub matrix_membership: CounterPair,
    pub matrix_avatar: CounterPair,
    pub ts_matrix_room_ok: TimeSeries,
    pub ts_matrix_room_fail: TimeSeries,
    pub ts_matrix_membership_ok: TimeSeries,
    pub ts_matrix_membership_fail: TimeSeries,
    pub ts_matrix_avatar_ok: TimeSeries,
    pub ts_matrix_avatar_fail: TimeSeries,

    // -- Chart 10: SMTP deliverability --
    pub smtp_otp: CounterPair,
    pub latency_smtp: LatencyHistogram,
    pub ts_smtp_ok: TimeSeries,
    pub ts_smtp_fail: TimeSeries,
    pub ts_smtp_p95: TimeSeries,

    // -- Sampler health metadata --
    pub last_sample_epoch: AtomicU32,
    pub sample_failures_total: AtomicU64,

    // -- Config callbacks --
    pub config: MetricsConfig,
}

impl MetricsStore {
    #[must_use]
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            req_total: AtomicU64::new(0),
            req_4xx: AtomicU64::new(0),
            req_5xx: AtomicU64::new(0),
            ts_req_rate: TimeSeries::new(),
            ts_4xx_rate: TimeSeries::new(),
            ts_5xx_rate: TimeSeries::new(),

            http_inflight: AtomicU64::new(0),
            ts_http_inflight: TimeSeries::new(),

            latency_auth: LatencyHistogram::new(),
            latency_profiles: LatencyHistogram::new(),
            latency_events: LatencyHistogram::new(),
            latency_uploads: LatencyHistogram::new(),
            latency_search: LatencyHistogram::new(),
            latency_matching: LatencyHistogram::new(),
            latency_matrix: LatencyHistogram::new(),
            ts_p95_auth: TimeSeries::new(),
            ts_p95_profiles: TimeSeries::new(),
            ts_p95_events: TimeSeries::new(),
            ts_p95_uploads: TimeSeries::new(),
            ts_p95_search: TimeSeries::new(),
            ts_p95_matching: TimeSeries::new(),
            ts_p95_matrix: TimeSeries::new(),

            ts_pool_size: TimeSeries::new(),
            ts_pool_available: TimeSeries::new(),
            ts_pool_waiting: TimeSeries::new(),

            latency_db_conn: LatencyHistogram::new(),
            ts_p95_db_conn: TimeSeries::new(),

            ts_outbox_pending: TimeSeries::new(),
            ts_outbox_ready: TimeSeries::new(),
            ts_outbox_retrying: TimeSeries::new(),
            ts_outbox_inflight: TimeSeries::new(),
            ts_outbox_failed: TimeSeries::new(),
            ts_outbox_oldest_ready_age: TimeSeries::new(),
            ts_outbox_oldest_pending_age: TimeSeries::new(),

            auth_cache_hits: AtomicU64::new(0),
            auth_cache_misses: AtomicU64::new(0),
            ts_auth_hit_rate: TimeSeries::new(),
            ts_auth_cache_entries: TimeSeries::new(),

            matrix_room_create: CounterPair::new(),
            matrix_membership: CounterPair::new(),
            matrix_avatar: CounterPair::new(),
            ts_matrix_room_ok: TimeSeries::new(),
            ts_matrix_room_fail: TimeSeries::new(),
            ts_matrix_membership_ok: TimeSeries::new(),
            ts_matrix_membership_fail: TimeSeries::new(),
            ts_matrix_avatar_ok: TimeSeries::new(),
            ts_matrix_avatar_fail: TimeSeries::new(),

            smtp_otp: CounterPair::new(),
            latency_smtp: LatencyHistogram::new(),
            ts_smtp_ok: TimeSeries::new(),
            ts_smtp_fail: TimeSeries::new(),
            ts_smtp_p95: TimeSeries::new(),

            last_sample_epoch: AtomicU32::new(0),
            sample_failures_total: AtomicU64::new(0),

            config,
        }
    }

    /// Get the latency histogram for a given endpoint group.
    pub const fn latency_for_group(&self, group: EndpointGroup) -> Option<&LatencyHistogram> {
        match group {
            EndpointGroup::Auth => Some(&self.latency_auth),
            EndpointGroup::Profiles => Some(&self.latency_profiles),
            EndpointGroup::Events => Some(&self.latency_events),
            EndpointGroup::Uploads => Some(&self.latency_uploads),
            EndpointGroup::Search => Some(&self.latency_search),
            EndpointGroup::Matching => Some(&self.latency_matching),
            EndpointGroup::Matrix => Some(&self.latency_matrix),
            EndpointGroup::Other => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{EndpointGroup, LatencyHistogram};

    #[test]
    fn drain_p95_resets_histogram_state() {
        let histogram = LatencyHistogram::new();
        histogram.record(Duration::from_millis(1));
        histogram.record(Duration::from_millis(400));

        assert_eq!(histogram.drain_p95(), 500.0);
        assert_eq!(histogram.drain_p95(), 0.0);
    }

    #[test]
    fn unrelated_routes_are_not_classified_as_auth() {
        assert!(matches!(
            EndpointGroup::from_path("/api/v1/catalog"),
            EndpointGroup::Other
        ));
    }
}
