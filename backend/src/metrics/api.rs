use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use super::collector::{LatencyHistogram, LatencyHistogramSnapshot, HISTOGRAM_BUCKETS};
use super::sampler::SAMPLE_INTERVAL_SECS;
use super::store::TimeSeries;
use crate::db::models::metrics_histogram_samples::MetricHistogramSample;
use crate::db::models::metrics_scalar_samples::MetricScalarSample;
use crate::db::schema::{metrics_histogram_samples, metrics_scalar_samples};

fn ops_status_token() -> Option<String> {
    std::env::var("OPS_STATUS_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn token_matches(actual: &str, expected: &str) -> bool {
    actual.as_bytes().ct_eq(expected.as_bytes()).into()
}

fn check_ops_token(headers: &HeaderMap) -> bool {
    let Some(expected) = ops_status_token() else {
        return false;
    };
    headers
        .get("x-ops-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| token_matches(actual, &expected))
}

fn now_epoch() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as u32)
        .unwrap_or(0)
}

#[derive(Deserialize)]
struct MetricsQuery {
    range: Option<String>,
}

#[derive(Serialize)]
struct SeriesData {
    timestamps: Vec<u32>,
    values: Vec<f32>,
}

#[derive(Serialize)]
struct NamedSeries {
    name: &'static str,
    #[serde(flatten)]
    data: SeriesData,
}

#[derive(Serialize)]
struct ChartData {
    label: &'static str,
    series: Vec<NamedSeries>,
}

#[derive(Serialize)]
struct MetricsMeta {
    source: &'static str,
    degraded: bool,
    sample_interval_seconds: u32,
    generated_at_epoch: u32,
    last_sample_epoch: Option<u32>,
    sample_failures_total: u64,
}

#[derive(Serialize)]
struct MetricsResponse {
    meta: MetricsMeta,
    charts: Vec<ChartData>,
}

type ScalarSeriesMap = HashMap<(i16, i16), BTreeMap<u32, f32>>;
type HistogramSeriesMap = HashMap<(i16, i16), BTreeMap<u32, LatencyHistogramSnapshot>>;

fn range_seconds(range: Option<&String>) -> i64 {
    match range.map(String::as_str) {
        Some("1h") => 3600,
        Some("6h") => 21600,
        _ => 86400,
    }
}

fn response_meta(source: &'static str, degraded: bool) -> Option<MetricsMeta> {
    let metrics = super::metrics()?;
    let last_sample_epoch = std::sync::atomic::AtomicU32::load(
        &metrics.last_sample_epoch,
        std::sync::atomic::Ordering::Relaxed,
    );
    Some(MetricsMeta {
        source,
        degraded,
        sample_interval_seconds: SAMPLE_INTERVAL_SECS,
        generated_at_epoch: now_epoch(),
        last_sample_epoch: if last_sample_epoch == 0 {
            None
        } else {
            Some(last_sample_epoch)
        },
        sample_failures_total: std::sync::atomic::AtomicU64::load(
            &metrics.sample_failures_total,
            std::sync::atomic::Ordering::Relaxed,
        ),
    })
}

fn read_series(ts: &TimeSeries, from: u32, to: u32) -> SeriesData {
    let (timestamps, values) = ts.read_range(from, to);
    SeriesData { timestamps, values }
}

fn named(name: &'static str, ts: &TimeSeries, from: u32, to: u32) -> NamedSeries {
    NamedSeries {
        name,
        data: read_series(ts, from, to),
    }
}

fn build_memory_charts(from: u32, to: u32) -> Option<Vec<ChartData>> {
    let metrics = super::metrics()?;
    Some(vec![
        ChartData {
            label: "Request Rate",
            series: vec![
                named("req/s", &metrics.ts_req_rate, from, to),
                named("4xx/s", &metrics.ts_4xx_rate, from, to),
                named("5xx/s", &metrics.ts_5xx_rate, from, to),
            ],
        },
        ChartData {
            label: "HTTP Concurrency",
            series: vec![named("inflight", &metrics.ts_http_inflight, from, to)],
        },
        ChartData {
            label: "p95 Latency (ms)",
            series: vec![
                named("auth", &metrics.ts_p95_auth, from, to),
                named("profiles", &metrics.ts_p95_profiles, from, to),
                named("events", &metrics.ts_p95_events, from, to),
                named("uploads", &metrics.ts_p95_uploads, from, to),
                named("search", &metrics.ts_p95_search, from, to),
                named("matching", &metrics.ts_p95_matching, from, to),
                named("matrix", &metrics.ts_p95_matrix, from, to),
            ],
        },
        ChartData {
            label: "DB Pool",
            series: vec![
                named("size", &metrics.ts_pool_size, from, to),
                named("available", &metrics.ts_pool_available, from, to),
                named("waiting", &metrics.ts_pool_waiting, from, to),
            ],
        },
        ChartData {
            label: "DB Conn p95 (ms)",
            series: vec![named("conn_acq", &metrics.ts_p95_db_conn, from, to)],
        },
        ChartData {
            label: "Outbox Queue",
            series: vec![
                named("pending", &metrics.ts_outbox_pending, from, to),
                named("ready", &metrics.ts_outbox_ready, from, to),
                named("retrying", &metrics.ts_outbox_retrying, from, to),
                named("inflight", &metrics.ts_outbox_inflight, from, to),
                named("failed", &metrics.ts_outbox_failed, from, to),
            ],
        },
        ChartData {
            label: "Outbox Lag (s)",
            series: vec![
                named(
                    "oldest_ready",
                    &metrics.ts_outbox_oldest_ready_age,
                    from,
                    to,
                ),
                named(
                    "oldest_pending",
                    &metrics.ts_outbox_oldest_pending_age,
                    from,
                    to,
                ),
            ],
        },
        ChartData {
            label: "Auth Cache",
            series: vec![
                named("hit%", &metrics.ts_auth_hit_rate, from, to),
                named("entries", &metrics.ts_auth_cache_entries, from, to),
            ],
        },
        ChartData {
            label: "Matrix Delivery",
            series: vec![
                named("room_ok", &metrics.ts_matrix_room_ok, from, to),
                named("room_fail", &metrics.ts_matrix_room_fail, from, to),
                named("membership_ok", &metrics.ts_matrix_membership_ok, from, to),
                named(
                    "membership_fail",
                    &metrics.ts_matrix_membership_fail,
                    from,
                    to,
                ),
                named("avatar_ok", &metrics.ts_matrix_avatar_ok, from, to),
                named("avatar_fail", &metrics.ts_matrix_avatar_fail, from, to),
            ],
        },
        ChartData {
            label: "SMTP Delivery",
            series: vec![
                named("otp_ok", &metrics.ts_smtp_ok, from, to),
                named("otp_fail", &metrics.ts_smtp_fail, from, to),
                named("p95_ms", &metrics.ts_smtp_p95, from, to),
            ],
        },
    ])
}

fn fallback_memory_response(from: u32, to: u32, degraded: bool) -> Response {
    let Some(meta) = response_meta("memory", degraded) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(charts) = build_memory_charts(from, to) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    Json(MetricsResponse { meta, charts }).into_response()
}

fn aggregate_scalar_rows(rows: Vec<MetricScalarSample>) -> ScalarSeriesMap {
    let mut map = HashMap::<(i16, i16), BTreeMap<u32, f32>>::new();
    for row in rows {
        let Ok(ts) = u32::try_from(row.ts.timestamp()) else {
            continue;
        };
        let series = map.entry((row.chart, row.series)).or_default();
        *series.entry(ts).or_default() += row.value;
    }
    map
}

fn aggregate_histogram_rows(rows: Vec<MetricHistogramSample>) -> HistogramSeriesMap {
    let mut map = HashMap::<(i16, i16), BTreeMap<u32, LatencyHistogramSnapshot>>::new();
    for row in rows {
        let Ok(ts) = u32::try_from(row.ts.timestamp()) else {
            continue;
        };
        let series = map.entry((row.chart, row.series)).or_default();
        let snapshot = series.entry(ts).or_default();
        let count = u64::try_from(row.count).unwrap_or(0);
        if row.bucket < HISTOGRAM_BUCKETS as i16 {
            snapshot.buckets[row.bucket as usize] += count;
        } else {
            snapshot.overflow += count;
        }
        snapshot.total += count;
    }
    map
}

fn scalar_named(
    name: &'static str,
    series: Option<&BTreeMap<u32, f32>>,
    transform: impl Fn(f32) -> f32,
) -> NamedSeries {
    let Some(series) = series else {
        return NamedSeries {
            name,
            data: SeriesData {
                timestamps: Vec::new(),
                values: Vec::new(),
            },
        };
    };
    let timestamps = series.keys().copied().collect();
    let values = series.values().copied().map(transform).collect();
    NamedSeries {
        name,
        data: SeriesData { timestamps, values },
    }
}

fn histogram_named(
    name: &'static str,
    series: Option<&BTreeMap<u32, LatencyHistogramSnapshot>>,
) -> NamedSeries {
    let Some(series) = series else {
        return NamedSeries {
            name,
            data: SeriesData {
                timestamps: Vec::new(),
                values: Vec::new(),
            },
        };
    };
    let timestamps = series.keys().copied().collect();
    let values = series
        .values()
        .map(LatencyHistogram::p95_from_snapshot)
        .collect();
    NamedSeries {
        name,
        data: SeriesData { timestamps, values },
    }
}

fn derived_hit_rate_series(
    hits: Option<&BTreeMap<u32, f32>>,
    misses: Option<&BTreeMap<u32, f32>>,
) -> NamedSeries {
    let mut merged = BTreeMap::<u32, f32>::new();

    if let Some(hits) = hits {
        for (&ts, &value) in hits {
            let misses_value = misses
                .and_then(|series| series.get(&ts))
                .copied()
                .unwrap_or(0.0);
            let total = value + misses_value;
            merged.insert(
                ts,
                if total > 0.0 {
                    (value / total) * 100.0
                } else {
                    0.0
                },
            );
        }
    }

    if let Some(misses) = misses {
        for &ts in misses.keys() {
            merged.entry(ts).or_insert(0.0);
        }
    }

    NamedSeries {
        name: "hit%",
        data: SeriesData {
            timestamps: merged.keys().copied().collect(),
            values: merged.values().copied().collect(),
        },
    }
}

fn build_timescaledb_charts(
    scalar_rows: Vec<MetricScalarSample>,
    histogram_rows: Vec<MetricHistogramSample>,
) -> Vec<ChartData> {
    let scalars = aggregate_scalar_rows(scalar_rows);
    let histograms = aggregate_histogram_rows(histogram_rows);

    vec![
        ChartData {
            label: "Request Rate",
            series: vec![
                scalar_named("req/s", scalars.get(&(0, 0)), |value| {
                    value / SAMPLE_INTERVAL_SECS as f32
                }),
                scalar_named("4xx/s", scalars.get(&(0, 1)), |value| {
                    value / SAMPLE_INTERVAL_SECS as f32
                }),
                scalar_named("5xx/s", scalars.get(&(0, 2)), |value| {
                    value / SAMPLE_INTERVAL_SECS as f32
                }),
            ],
        },
        ChartData {
            label: "HTTP Concurrency",
            series: vec![scalar_named("inflight", scalars.get(&(1, 0)), |value| {
                value
            })],
        },
        ChartData {
            label: "p95 Latency (ms)",
            series: vec![
                histogram_named("auth", histograms.get(&(2, 0))),
                histogram_named("profiles", histograms.get(&(2, 1))),
                histogram_named("events", histograms.get(&(2, 2))),
                histogram_named("uploads", histograms.get(&(2, 3))),
                histogram_named("search", histograms.get(&(2, 4))),
                histogram_named("matching", histograms.get(&(2, 5))),
                histogram_named("matrix", histograms.get(&(2, 6))),
            ],
        },
        ChartData {
            label: "DB Pool",
            series: vec![
                scalar_named("size", scalars.get(&(3, 0)), |value| value),
                scalar_named("available", scalars.get(&(3, 1)), |value| value),
                scalar_named("waiting", scalars.get(&(3, 2)), |value| value),
            ],
        },
        ChartData {
            label: "DB Conn p95 (ms)",
            series: vec![histogram_named("conn_acq", histograms.get(&(4, 0)))],
        },
        ChartData {
            label: "Outbox Queue",
            series: vec![
                scalar_named("pending", scalars.get(&(5, 0)), |value| value),
                scalar_named("ready", scalars.get(&(5, 1)), |value| value),
                scalar_named("retrying", scalars.get(&(5, 2)), |value| value),
                scalar_named("inflight", scalars.get(&(5, 3)), |value| value),
                scalar_named("failed", scalars.get(&(5, 4)), |value| value),
            ],
        },
        ChartData {
            label: "Outbox Lag (s)",
            series: vec![
                scalar_named("oldest_ready", scalars.get(&(6, 0)), |value| value),
                scalar_named("oldest_pending", scalars.get(&(6, 1)), |value| value),
            ],
        },
        ChartData {
            label: "Auth Cache",
            series: vec![
                derived_hit_rate_series(scalars.get(&(7, 0)), scalars.get(&(7, 1))),
                scalar_named("entries", scalars.get(&(7, 2)), |value| value),
            ],
        },
        ChartData {
            label: "Matrix Delivery",
            series: vec![
                scalar_named("room_ok", scalars.get(&(8, 0)), |value| value),
                scalar_named("room_fail", scalars.get(&(8, 1)), |value| value),
                scalar_named("membership_ok", scalars.get(&(8, 2)), |value| value),
                scalar_named("membership_fail", scalars.get(&(8, 3)), |value| value),
                scalar_named("avatar_ok", scalars.get(&(8, 4)), |value| value),
                scalar_named("avatar_fail", scalars.get(&(8, 5)), |value| value),
            ],
        },
        ChartData {
            label: "SMTP Delivery",
            series: vec![
                scalar_named("otp_ok", scalars.get(&(9, 0)), |value| value),
                scalar_named("otp_fail", scalars.get(&(9, 1)), |value| value),
                histogram_named("p95_ms", histograms.get(&(9, 2))),
            ],
        },
    ]
}

async fn metrics_handler(headers: HeaderMap, Query(query): Query<MetricsQuery>) -> Response {
    if !check_ops_token(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if super::metrics().is_none() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let now = now_epoch();
    let range = range_seconds(query.range.as_ref());
    let from_epoch = now.saturating_sub(range as u32);
    let from = Utc::now() - Duration::seconds(range);

    let mut conn = match crate::db::conn().await {
        Ok(conn) => conn,
        Err(_) => return fallback_memory_response(from_epoch, now, true),
    };

    let scalar_rows: Vec<MetricScalarSample> = match metrics_scalar_samples::table
        .filter(metrics_scalar_samples::ts.ge(from))
        .order((
            metrics_scalar_samples::chart.asc(),
            metrics_scalar_samples::series.asc(),
            metrics_scalar_samples::ts.asc(),
        ))
        .load(&mut conn)
        .await
    {
        Ok(rows) => rows,
        Err(_) => return fallback_memory_response(from_epoch, now, true),
    };

    let histogram_rows: Vec<MetricHistogramSample> = match metrics_histogram_samples::table
        .filter(metrics_histogram_samples::ts.ge(from))
        .order((
            metrics_histogram_samples::chart.asc(),
            metrics_histogram_samples::series.asc(),
            metrics_histogram_samples::bucket.asc(),
            metrics_histogram_samples::ts.asc(),
        ))
        .load(&mut conn)
        .await
    {
        Ok(rows) => rows,
        Err(_) => return fallback_memory_response(from_epoch, now, true),
    };

    let Some(meta) = response_meta("timescaledb", false) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let charts = build_timescaledb_charts(scalar_rows, histogram_rows);
    Json(MetricsResponse { meta, charts }).into_response()
}

pub fn routes() -> Router {
    Router::new().route("/api/v1/metrics", get(metrics_handler))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{aggregate_histogram_rows, aggregate_scalar_rows, build_timescaledb_charts};
    use crate::db::models::metrics_histogram_samples::MetricHistogramSample;
    use crate::db::models::metrics_scalar_samples::MetricScalarSample;

    #[test]
    fn scalar_rows_sum_across_instances() {
        let ts = Utc.timestamp_opt(1_741_261_600, 0).single().unwrap();
        let rows = vec![
            MetricScalarSample {
                ts,
                instance_id: String::from("a"),
                chart: 0,
                series: 0,
                value: 12.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("b"),
                chart: 0,
                series: 0,
                value: 8.0,
            },
        ];

        let aggregated = aggregate_scalar_rows(rows);
        let series = aggregated.get(&(0, 0)).unwrap();
        assert_eq!(*series.values().next().unwrap(), 20.0);
    }

    #[test]
    fn charts_merge_instances_and_derive_latency_and_hit_rate() {
        let ts = Utc.timestamp_opt(1_741_261_600, 0).single().unwrap();
        let scalar_rows = vec![
            MetricScalarSample {
                ts,
                instance_id: String::from("a"),
                chart: 7,
                series: 0,
                value: 9.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("a"),
                chart: 7,
                series: 1,
                value: 1.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("b"),
                chart: 7,
                series: 0,
                value: 8.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("b"),
                chart: 7,
                series: 1,
                value: 2.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("a"),
                chart: 7,
                series: 2,
                value: 4.0,
            },
            MetricScalarSample {
                ts,
                instance_id: String::from("b"),
                chart: 7,
                series: 2,
                value: 6.0,
            },
        ];
        let histogram_rows = vec![
            MetricHistogramSample {
                ts,
                instance_id: String::from("a"),
                chart: 2,
                series: 0,
                bucket: 0,
                count: 95,
            },
            MetricHistogramSample {
                ts,
                instance_id: String::from("b"),
                chart: 2,
                series: 0,
                bucket: 10,
                count: 5,
            },
        ];

        let aggregated_histograms = aggregate_histogram_rows(vec![
            MetricHistogramSample {
                ts,
                instance_id: String::from("a"),
                chart: 2,
                series: 0,
                bucket: 0,
                count: 95,
            },
            MetricHistogramSample {
                ts,
                instance_id: String::from("b"),
                chart: 2,
                series: 0,
                bucket: 10,
                count: 5,
            },
        ]);
        assert_eq!(
            aggregated_histograms
                .get(&(2, 0))
                .unwrap()
                .values()
                .next()
                .unwrap()
                .total,
            100
        );

        let charts = build_timescaledb_charts(scalar_rows, histogram_rows);
        let auth_cache_chart = &charts[7];
        assert_eq!(auth_cache_chart.series[0].data.values, vec![85.0]);
        assert_eq!(auth_cache_chart.series[1].data.values, vec![10.0]);

        let latency_chart = &charts[2];
        assert_eq!(latency_chart.series[0].data.values, vec![1.0]);
    }
}
