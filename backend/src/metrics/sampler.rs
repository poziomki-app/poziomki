use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use diesel_async::RunQueryDsl;

use super::collector::{LatencyHistogramSnapshot, MetricsStore, HISTOGRAM_BUCKETS};
use crate::db::models::metrics_histogram_samples::NewMetricHistogramSample;
use crate::db::models::metrics_scalar_samples::NewMetricScalarSample;
use crate::db::schema::{metrics_histogram_samples, metrics_scalar_samples};

pub const SAMPLE_INTERVAL_SECS: u32 = 10;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(SAMPLE_INTERVAL_SECS as u64);

#[derive(Clone, Copy)]
struct ScalarSample {
    chart: i16,
    series: i16,
    value: f32,
}

struct HistogramSample {
    chart: i16,
    series: i16,
    snapshot: LatencyHistogramSnapshot,
}

fn now_epoch() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as u32)
        .unwrap_or(0)
}

pub fn spawn_sampler(store: &'static MetricsStore) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.tick().await;

        loop {
            interval.tick().await;
            sample(store).await;
        }
    });
}

async fn sample(store: &MetricsStore) {
    let ts_epoch = now_epoch();
    let ts = Utc::now();
    let mut scalar_samples = Vec::with_capacity(32);
    let mut histogram_samples = Vec::with_capacity(10);

    sample_request_rates(store, ts_epoch, &mut scalar_samples);
    sample_http_inflight(store, ts_epoch, &mut scalar_samples);
    sample_latencies(store, ts_epoch, &mut histogram_samples);
    sample_pool(store, ts_epoch, &mut scalar_samples);
    sample_db_latency(store, ts_epoch, &mut histogram_samples);
    sample_outbox(store, ts_epoch, &mut scalar_samples).await;
    sample_auth_cache(store, ts_epoch, &mut scalar_samples);
    sample_matrix(store, ts_epoch, &mut scalar_samples);
    sample_smtp(store, ts_epoch, &mut scalar_samples, &mut histogram_samples);

    store.last_sample_epoch.store(ts_epoch, Ordering::Relaxed);

    if persist_samples(store, ts, scalar_samples, histogram_samples)
        .await
        .is_err()
    {
        store.sample_failures_total.fetch_add(1, Ordering::Relaxed);
    }
}

async fn persist_samples(
    store: &MetricsStore,
    ts: chrono::DateTime<Utc>,
    scalar_samples: Vec<ScalarSample>,
    histogram_samples: Vec<HistogramSample>,
) -> Result<(), ()> {
    if scalar_samples.is_empty() && histogram_samples.is_empty() {
        return Ok(());
    }

    let mut conn = crate::db::conn().await.map_err(|_| ())?;
    let instance_id = store.instance_id.clone();

    if !scalar_samples.is_empty() {
        let rows: Vec<NewMetricScalarSample> = scalar_samples
            .into_iter()
            .map(|sample| NewMetricScalarSample {
                ts,
                instance_id: instance_id.clone(),
                chart: sample.chart,
                series: sample.series,
                value: sample.value,
            })
            .collect();

        diesel::insert_into(metrics_scalar_samples::table)
            .values(&rows)
            .execute(&mut conn)
            .await
            .map_err(|_| ())?;
    }

    if !histogram_samples.is_empty() {
        let mut rows = Vec::new();
        for sample in histogram_samples {
            for (bucket_index, count) in sample.snapshot.buckets.into_iter().enumerate() {
                if count == 0 {
                    continue;
                }
                rows.push(NewMetricHistogramSample {
                    ts,
                    instance_id: instance_id.clone(),
                    chart: sample.chart,
                    series: sample.series,
                    bucket: bucket_index as i16,
                    count: count as i64,
                });
            }

            if sample.snapshot.overflow > 0 {
                rows.push(NewMetricHistogramSample {
                    ts,
                    instance_id: instance_id.clone(),
                    chart: sample.chart,
                    series: sample.series,
                    bucket: HISTOGRAM_BUCKETS as i16,
                    count: sample.snapshot.overflow as i64,
                });
            }
        }

        if !rows.is_empty() {
            diesel::insert_into(metrics_histogram_samples::table)
                .values(&rows)
                .execute(&mut conn)
                .await
                .map_err(|_| ())?;
        }
    }

    Ok(())
}

fn sample_request_rates(
    store: &MetricsStore,
    ts_epoch: u32,
    scalar_samples: &mut Vec<ScalarSample>,
) {
    let total = store.req_total.swap(0, Ordering::Relaxed);
    let four_xx = store.req_4xx.swap(0, Ordering::Relaxed);
    let five_xx = store.req_5xx.swap(0, Ordering::Relaxed);

    store
        .ts_req_rate
        .push(ts_epoch, total as f32 / SAMPLE_INTERVAL_SECS as f32);
    store
        .ts_4xx_rate
        .push(ts_epoch, four_xx as f32 / SAMPLE_INTERVAL_SECS as f32);
    store
        .ts_5xx_rate
        .push(ts_epoch, five_xx as f32 / SAMPLE_INTERVAL_SECS as f32);

    scalar_samples.push(ScalarSample {
        chart: 0,
        series: 0,
        value: total as f32,
    });
    scalar_samples.push(ScalarSample {
        chart: 0,
        series: 1,
        value: four_xx as f32,
    });
    scalar_samples.push(ScalarSample {
        chart: 0,
        series: 2,
        value: five_xx as f32,
    });
}

fn sample_http_inflight(
    store: &MetricsStore,
    ts_epoch: u32,
    scalar_samples: &mut Vec<ScalarSample>,
) {
    let inflight =
        std::sync::atomic::AtomicU64::load(&store.http_inflight, Ordering::Relaxed) as f32;
    store.ts_http_inflight.push(ts_epoch, inflight);
    scalar_samples.push(ScalarSample {
        chart: 1,
        series: 0,
        value: inflight,
    });
}

fn sample_latencies(
    store: &MetricsStore,
    ts_epoch: u32,
    histogram_samples: &mut Vec<HistogramSample>,
) {
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_auth,
        2,
        0,
        store.latency_auth.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_profiles,
        2,
        1,
        store.latency_profiles.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_events,
        2,
        2,
        store.latency_events.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_uploads,
        2,
        3,
        store.latency_uploads.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_search,
        2,
        4,
        store.latency_search.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_matching,
        2,
        5,
        store.latency_matching.drain_snapshot(),
        histogram_samples,
    );
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_matrix,
        2,
        6,
        store.latency_matrix.drain_snapshot(),
        histogram_samples,
    );
}

fn sample_latency_group(
    ts_epoch: u32,
    series: &super::store::TimeSeries,
    chart: i16,
    series_id: i16,
    snapshot: LatencyHistogramSnapshot,
    histogram_samples: &mut Vec<HistogramSample>,
) {
    series.push(
        ts_epoch,
        super::collector::LatencyHistogram::p95_from_snapshot(&snapshot),
    );
    histogram_samples.push(HistogramSample {
        chart,
        series: series_id,
        snapshot,
    });
}

fn sample_pool(store: &MetricsStore, ts_epoch: u32, scalar_samples: &mut Vec<ScalarSample>) {
    if let Some((size, available, waiting)) = (store.config.pool_status)() {
        store.ts_pool_size.push(ts_epoch, size as f32);
        store.ts_pool_available.push(ts_epoch, available as f32);
        store.ts_pool_waiting.push(ts_epoch, waiting as f32);

        scalar_samples.push(ScalarSample {
            chart: 3,
            series: 0,
            value: size as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 3,
            series: 1,
            value: available as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 3,
            series: 2,
            value: waiting as f32,
        });
    }
}

fn sample_db_latency(
    store: &MetricsStore,
    ts_epoch: u32,
    histogram_samples: &mut Vec<HistogramSample>,
) {
    sample_latency_group(
        ts_epoch,
        &store.ts_p95_db_conn,
        4,
        0,
        store.latency_db_conn.drain_snapshot(),
        histogram_samples,
    );
}

async fn sample_outbox(
    store: &MetricsStore,
    ts_epoch: u32,
    scalar_samples: &mut Vec<ScalarSample>,
) {
    let future = (store.config.outbox_snapshot)();
    if let Some(snapshot) = future.await {
        store
            .ts_outbox_pending
            .push(ts_epoch, snapshot.pending_jobs as f32);
        store
            .ts_outbox_ready
            .push(ts_epoch, snapshot.ready_jobs as f32);
        store
            .ts_outbox_retrying
            .push(ts_epoch, snapshot.retrying_jobs as f32);
        store
            .ts_outbox_inflight
            .push(ts_epoch, snapshot.inflight_jobs as f32);
        store
            .ts_outbox_failed
            .push(ts_epoch, snapshot.failed_jobs as f32);
        store
            .ts_outbox_oldest_ready_age
            .push(ts_epoch, snapshot.oldest_ready_job_age_seconds as f32);
        store
            .ts_outbox_oldest_pending_age
            .push(ts_epoch, snapshot.oldest_pending_job_age_seconds as f32);

        scalar_samples.push(ScalarSample {
            chart: 5,
            series: 0,
            value: snapshot.pending_jobs as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 5,
            series: 1,
            value: snapshot.ready_jobs as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 5,
            series: 2,
            value: snapshot.retrying_jobs as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 5,
            series: 3,
            value: snapshot.inflight_jobs as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 5,
            series: 4,
            value: snapshot.failed_jobs as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 6,
            series: 0,
            value: snapshot.oldest_ready_job_age_seconds as f32,
        });
        scalar_samples.push(ScalarSample {
            chart: 6,
            series: 1,
            value: snapshot.oldest_pending_job_age_seconds as f32,
        });
    }
}

fn sample_auth_cache(store: &MetricsStore, ts_epoch: u32, scalar_samples: &mut Vec<ScalarSample>) {
    let hits = store.auth_cache_hits.swap(0, Ordering::Relaxed);
    let misses = store.auth_cache_misses.swap(0, Ordering::Relaxed);
    let total = hits + misses;
    let hit_rate = if total > 0 {
        (hits as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    let entry_count = (store.config.auth_cache_size)() as f32;

    store.ts_auth_hit_rate.push(ts_epoch, hit_rate);
    store.ts_auth_cache_entries.push(ts_epoch, entry_count);

    scalar_samples.push(ScalarSample {
        chart: 7,
        series: 0,
        value: hits as f32,
    });
    scalar_samples.push(ScalarSample {
        chart: 7,
        series: 1,
        value: misses as f32,
    });
    scalar_samples.push(ScalarSample {
        chart: 7,
        series: 2,
        value: entry_count,
    });
}

fn sample_matrix(store: &MetricsStore, ts_epoch: u32, scalar_samples: &mut Vec<ScalarSample>) {
    let (room_ok, room_fail) = store.matrix_room_create.drain();
    let (membership_ok, membership_fail) = store.matrix_membership.drain();
    let (avatar_ok, avatar_fail) = store.matrix_avatar.drain();

    store.ts_matrix_room_ok.push(ts_epoch, room_ok as f32);
    store.ts_matrix_room_fail.push(ts_epoch, room_fail as f32);
    store
        .ts_matrix_membership_ok
        .push(ts_epoch, membership_ok as f32);
    store
        .ts_matrix_membership_fail
        .push(ts_epoch, membership_fail as f32);
    store.ts_matrix_avatar_ok.push(ts_epoch, avatar_ok as f32);
    store
        .ts_matrix_avatar_fail
        .push(ts_epoch, avatar_fail as f32);

    for (series, value) in [
        (0, room_ok),
        (1, room_fail),
        (2, membership_ok),
        (3, membership_fail),
        (4, avatar_ok),
        (5, avatar_fail),
    ] {
        scalar_samples.push(ScalarSample {
            chart: 8,
            series,
            value: value as f32,
        });
    }
}

fn sample_smtp(
    store: &MetricsStore,
    ts_epoch: u32,
    scalar_samples: &mut Vec<ScalarSample>,
    histogram_samples: &mut Vec<HistogramSample>,
) {
    let (ok, fail) = store.smtp_otp.drain();
    let snapshot = store.latency_smtp.drain_snapshot();

    store.ts_smtp_ok.push(ts_epoch, ok as f32);
    store.ts_smtp_fail.push(ts_epoch, fail as f32);
    store.ts_smtp_p95.push(
        ts_epoch,
        super::collector::LatencyHistogram::p95_from_snapshot(&snapshot),
    );

    scalar_samples.push(ScalarSample {
        chart: 9,
        series: 0,
        value: ok as f32,
    });
    scalar_samples.push(ScalarSample {
        chart: 9,
        series: 1,
        value: fail as f32,
    });
    histogram_samples.push(HistogramSample {
        chart: 9,
        series: 2,
        snapshot,
    });
}
