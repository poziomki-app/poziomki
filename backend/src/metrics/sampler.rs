use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::collector::MetricsStore;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(10);

fn now_epoch() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

pub fn spawn_sampler(store: &'static MetricsStore) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        // First tick is immediate — skip it so we don't push a zero-point
        interval.tick().await;

        loop {
            interval.tick().await;
            sample(store).await;
        }
    });
}

async fn sample(m: &MetricsStore) {
    let ts = now_epoch();

    sample_request_rates(m, ts);
    sample_http_inflight(m, ts);
    sample_latencies(m, ts);
    sample_pool(m, ts);
    sample_db_latency(m, ts);
    sample_outbox(m, ts).await;
    sample_auth_cache(m, ts);
    sample_matrix(m, ts);
    sample_smtp(m, ts);
    m.last_sample_epoch.store(ts, Ordering::Relaxed);
}

fn sample_request_rates(m: &MetricsStore, ts: u32) {
    let total = m.req_total.swap(0, Ordering::Relaxed);
    let fourxx = m.req_4xx.swap(0, Ordering::Relaxed);
    let fivexx = m.req_5xx.swap(0, Ordering::Relaxed);

    // Convert to per-second rates (interval = 10s)
    let rate = total as f32 / 10.0;
    let rate_4xx = fourxx as f32 / 10.0;
    let rate_5xx = fivexx as f32 / 10.0;

    m.ts_req_rate.push(ts, rate);
    m.ts_4xx_rate.push(ts, rate_4xx);
    m.ts_5xx_rate.push(ts, rate_5xx);
}

fn sample_http_inflight(m: &MetricsStore, ts: u32) {
    m.ts_http_inflight
        .push(ts, m.http_inflight.load(Ordering::Relaxed) as f32);
}

fn sample_latencies(m: &MetricsStore, ts: u32) {
    m.ts_p95_auth.push(ts, m.latency_auth.drain_p95());
    m.ts_p95_profiles.push(ts, m.latency_profiles.drain_p95());
    m.ts_p95_events.push(ts, m.latency_events.drain_p95());
    m.ts_p95_uploads.push(ts, m.latency_uploads.drain_p95());
    m.ts_p95_search.push(ts, m.latency_search.drain_p95());
    m.ts_p95_matching.push(ts, m.latency_matching.drain_p95());
    m.ts_p95_matrix.push(ts, m.latency_matrix.drain_p95());
}

fn sample_pool(m: &MetricsStore, ts: u32) {
    if let Some((size, available, waiting)) = (m.config.pool_status)() {
        m.ts_pool_size.push(ts, size as f32);
        m.ts_pool_available.push(ts, available as f32);
        m.ts_pool_waiting.push(ts, waiting as f32);
    }
}

fn sample_db_latency(m: &MetricsStore, ts: u32) {
    m.ts_p95_db_conn.push(ts, m.latency_db_conn.drain_p95());
}

async fn sample_outbox(m: &MetricsStore, ts: u32) {
    let future = (m.config.outbox_snapshot)();
    if let Some(snapshot) = future.await {
        m.ts_outbox_pending.push(ts, snapshot.pending_jobs as f32);
        m.ts_outbox_ready.push(ts, snapshot.ready_jobs as f32);
        m.ts_outbox_retrying.push(ts, snapshot.retrying_jobs as f32);
        m.ts_outbox_inflight.push(ts, snapshot.inflight_jobs as f32);
        m.ts_outbox_failed.push(ts, snapshot.failed_jobs as f32);
        m.ts_outbox_oldest_ready_age
            .push(ts, snapshot.oldest_ready_job_age_seconds as f32);
        m.ts_outbox_oldest_pending_age
            .push(ts, snapshot.oldest_pending_job_age_seconds as f32);
    }
}

fn sample_auth_cache(m: &MetricsStore, ts: u32) {
    let hits = m.auth_cache_hits.swap(0, Ordering::Relaxed);
    let misses = m.auth_cache_misses.swap(0, Ordering::Relaxed);
    let total = hits + misses;
    let hit_rate = if total > 0 {
        (hits as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    m.ts_auth_hit_rate.push(ts, hit_rate);

    let entry_count = (m.config.auth_cache_size)();
    m.ts_auth_cache_entries.push(ts, entry_count as f32);
}

fn sample_matrix(m: &MetricsStore, ts: u32) {
    let (room_ok, room_fail) = m.matrix_room_create.drain();
    m.ts_matrix_room_ok.push(ts, room_ok as f32);
    m.ts_matrix_room_fail.push(ts, room_fail as f32);

    let (mem_ok, mem_fail) = m.matrix_membership.drain();
    m.ts_matrix_membership_ok.push(ts, mem_ok as f32);
    m.ts_matrix_membership_fail.push(ts, mem_fail as f32);

    let (av_ok, av_fail) = m.matrix_avatar.drain();
    m.ts_matrix_avatar_ok.push(ts, av_ok as f32);
    m.ts_matrix_avatar_fail.push(ts, av_fail as f32);
}

fn sample_smtp(m: &MetricsStore, ts: u32) {
    let (ok, fail) = m.smtp_otp.drain();
    m.ts_smtp_ok.push(ts, ok as f32);
    m.ts_smtp_fail.push(ts, fail as f32);
    m.ts_smtp_p95.push(ts, m.latency_smtp.drain_p95());
}
