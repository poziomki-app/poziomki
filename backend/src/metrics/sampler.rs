#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::sync::atomic::Ordering;
use std::time::Duration;

use chrono::Utc;
use diesel_async::RunQueryDsl;

use super::collector::{
    p95_from_snapshot, LatencyHistogramSnapshot, MetricsStore, HISTOGRAM_BUCKETS,
};
use crate::db::models::metrics_samples::{NewHistogramSample, NewScalarSample};
use crate::db::schema::{metrics_histogram_samples, metrics_scalar_samples};

pub const SAMPLE_INTERVAL_SECS: u32 = 10;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(SAMPLE_INTERVAL_SECS as u64);

struct ScalarEntry {
    chart: i16,
    series: i16,
    value: f32,
}

struct HistogramEntry {
    chart: i16,
    series: i16,
    snapshot: LatencyHistogramSnapshot,
}

struct SampleBatch {
    scalars: Vec<ScalarEntry>,
    histograms: Vec<HistogramEntry>,
}

impl SampleBatch {
    fn new() -> Self {
        Self {
            scalars: Vec::with_capacity(24),
            histograms: Vec::with_capacity(8),
        }
    }

    fn scalar(&mut self, chart: i16, series: i16, value: f32) {
        self.scalars.push(ScalarEntry {
            chart,
            series,
            value,
        });
    }

    fn histogram(&mut self, chart: i16, series: i16, snapshot: LatencyHistogramSnapshot) {
        self.histograms.push(HistogramEntry {
            chart,
            series,
            snapshot,
        });
    }
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
    let ts = super::now_epoch();
    let mut batch = SampleBatch::new();

    sample_request_rates(m, ts, &mut batch);
    sample_latencies(m, ts, &mut batch);
    sample_pool(m, ts, &mut batch);
    sample_db_latency(m, ts, &mut batch);
    sample_outbox(m, ts, &mut batch).await;
    sample_auth_cache(m, ts, &mut batch);
    sample_matrix(m, ts, &mut batch);
    sample_smtp(m, ts, &mut batch);

    persist_samples(m, batch).await;
    m.last_sample_epoch.store(ts, Ordering::Relaxed);
}

fn sample_request_rates(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let total = m.req_total.swap(0, Ordering::Relaxed);
    let fourxx = m.req_4xx.swap(0, Ordering::Relaxed);
    let fivexx = m.req_5xx.swap(0, Ordering::Relaxed);

    // Ring buffer gets per-second rates
    let rate = total as f32 / SAMPLE_INTERVAL_SECS as f32;
    let rate_4xx = fourxx as f32 / SAMPLE_INTERVAL_SECS as f32;
    let rate_5xx = fivexx as f32 / SAMPLE_INTERVAL_SECS as f32;

    m.ts_req_rate.push(ts, rate);
    m.ts_4xx_rate.push(ts, rate_4xx);
    m.ts_5xx_rate.push(ts, rate_5xx);

    // DB gets raw counts
    batch.scalar(0, 0, total as f32);
    batch.scalar(0, 1, fourxx as f32);
    batch.scalar(0, 2, fivexx as f32);
}

fn sample_latencies(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let pairs: [(
        i16,
        &super::collector::LatencyHistogram,
        &super::store::TimeSeries,
    ); 7] = [
        (0, &m.latency_auth, &m.ts_p95_auth),
        (1, &m.latency_profiles, &m.ts_p95_profiles),
        (2, &m.latency_events, &m.ts_p95_events),
        (3, &m.latency_uploads, &m.ts_p95_uploads),
        (4, &m.latency_search, &m.ts_p95_search),
        (5, &m.latency_matching, &m.ts_p95_matching),
        (6, &m.latency_matrix, &m.ts_p95_matrix),
    ];

    for (series, histogram, ts_series) in pairs {
        let snap = histogram.drain_snapshot();
        ts_series.push(ts, p95_from_snapshot(&snap));
        batch.histogram(1, series, snap);
    }
}

fn sample_pool(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    if let Some((size, available, waiting)) = (m.config.pool_status)() {
        m.ts_pool_size.push(ts, size as f32);
        m.ts_pool_available.push(ts, available as f32);
        m.ts_pool_waiting.push(ts, waiting as f32);

        batch.scalar(2, 0, size as f32);
        batch.scalar(2, 1, available as f32);
        batch.scalar(2, 2, waiting as f32);
    }
}

fn sample_db_latency(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let snap = m.latency_db_conn.drain_snapshot();
    m.ts_p95_db_conn.push(ts, p95_from_snapshot(&snap));
    batch.histogram(3, 0, snap);
}

async fn sample_outbox(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let future = (m.config.outbox_snapshot)();
    if let Some((pending, ready, retrying, inflight, failed)) = future.await {
        m.ts_outbox_pending.push(ts, pending as f32);
        m.ts_outbox_ready.push(ts, ready as f32);
        m.ts_outbox_retrying.push(ts, retrying as f32);
        m.ts_outbox_inflight.push(ts, inflight as f32);
        m.ts_outbox_failed.push(ts, failed as f32);

        batch.scalar(4, 0, pending as f32);
        batch.scalar(4, 1, ready as f32);
        batch.scalar(4, 2, retrying as f32);
        batch.scalar(4, 3, inflight as f32);
        batch.scalar(4, 4, failed as f32);
    }
}

fn sample_auth_cache(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
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

    // DB gets raw counts
    batch.scalar(5, 0, hits as f32);
    batch.scalar(5, 1, misses as f32);
    batch.scalar(5, 2, entry_count as f32);
}

fn sample_matrix(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let (room_ok, room_fail) = m.matrix_room_create.drain();
    m.ts_matrix_room_ok.push(ts, room_ok as f32);
    m.ts_matrix_room_fail.push(ts, room_fail as f32);

    let (mem_ok, mem_fail) = m.matrix_membership.drain();
    m.ts_matrix_membership_ok.push(ts, mem_ok as f32);
    m.ts_matrix_membership_fail.push(ts, mem_fail as f32);

    let (av_ok, av_fail) = m.matrix_avatar.drain();
    m.ts_matrix_avatar_ok.push(ts, av_ok as f32);
    m.ts_matrix_avatar_fail.push(ts, av_fail as f32);

    batch.scalar(6, 0, room_ok as f32);
    batch.scalar(6, 1, room_fail as f32);
    batch.scalar(6, 2, mem_ok as f32);
    batch.scalar(6, 3, mem_fail as f32);
    batch.scalar(6, 4, av_ok as f32);
    batch.scalar(6, 5, av_fail as f32);
}

fn sample_smtp(m: &MetricsStore, ts: u32, batch: &mut SampleBatch) {
    let (ok, fail) = m.smtp_otp.drain();
    m.ts_smtp_ok.push(ts, ok as f32);
    m.ts_smtp_fail.push(ts, fail as f32);

    let snap = m.latency_smtp.drain_snapshot();
    m.ts_smtp_p95.push(ts, p95_from_snapshot(&snap));

    batch.scalar(7, 0, ok as f32);
    batch.scalar(7, 1, fail as f32);
    batch.histogram(7, 2, snap);
}

async fn persist_samples(m: &MetricsStore, batch: SampleBatch) {
    // Skip DB writes when everything is zero (idle server)
    let all_zero = batch.scalars.iter().all(|e| e.value == 0.0)
        && batch.histograms.iter().all(|e| e.snapshot.total == 0);
    if all_zero {
        return;
    }

    let Ok(conn) = crate::db::conn().await else {
        m.sample_failures_total.fetch_add(1, Ordering::Relaxed);
        return;
    };

    if let Err(e) = do_persist(&m.config.instance_id, batch, conn).await {
        tracing::debug!(error = %e, "metrics persist failed");
        m.sample_failures_total.fetch_add(1, Ordering::Relaxed);
    }
}

/// Sentinel bucket index for overflow (>5s) samples.
const OVERFLOW_BUCKET: i16 = HISTOGRAM_BUCKETS as i16;

async fn do_persist(
    instance_id: &str,
    batch: SampleBatch,
    mut conn: crate::db::DbConn,
) -> Result<(), diesel::result::Error> {
    let now = Utc::now();
    let id = instance_id.to_owned();

    if !batch.scalars.is_empty() {
        let rows: Vec<NewScalarSample> = batch
            .scalars
            .iter()
            .map(|e| NewScalarSample {
                ts: now,
                instance_id: id.clone(),
                chart: e.chart,
                series: e.series,
                value: e.value,
            })
            .collect();

        diesel::insert_into(metrics_scalar_samples::table)
            .values(&rows)
            .execute(&mut *conn)
            .await?;
    }

    let histogram_rows: Vec<NewHistogramSample> = batch
        .histograms
        .iter()
        .flat_map(|e| {
            let id = id.clone();
            let bucket_rows = (0..HISTOGRAM_BUCKETS).filter_map(move |i| {
                let &count = e.snapshot.buckets.get(i)?;
                (count > 0).then_some((i as i16, count as i64))
            });

            let overflow_row = if e.snapshot.overflow > 0 {
                Some((OVERFLOW_BUCKET, e.snapshot.overflow as i64))
            } else {
                None
            };

            bucket_rows
                .chain(overflow_row)
                .map(move |(bucket, count)| NewHistogramSample {
                    ts: now,
                    instance_id: id.clone(),
                    chart: e.chart,
                    series: e.series,
                    bucket,
                    count,
                })
        })
        .collect();

    if !histogram_rows.is_empty() {
        diesel::insert_into(metrics_histogram_samples::table)
            .values(&histogram_rows)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}
