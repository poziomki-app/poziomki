#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::type_complexity
)]

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subtle::ConstantTimeEq;

use super::collector::{
    p95_from_snapshot, LatencyHistogramSnapshot, MetricsStore, HISTOGRAM_BUCKETS,
};
use super::sampler::SAMPLE_INTERVAL_SECS;
use super::store::TimeSeries;
use crate::api::env_non_empty;
use crate::db::models::metrics_samples::{HistogramSample, ScalarSample};

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

fn check_ops_token(headers: &HeaderMap) -> bool {
    let Some(expected) = env_non_empty("OPS_STATUS_TOKEN") else {
        return false;
    };
    headers
        .get("x-ops-token")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|actual| actual.as_bytes().ct_eq(expected.as_bytes()).into())
}

fn check_ops_token_value(token: &str) -> bool {
    let Some(expected) = env_non_empty("OPS_STATUS_TOKEN") else {
        return false;
    };
    token.as_bytes().ct_eq(expected.as_bytes()).into()
}

#[derive(Deserialize)]
struct DashboardQuery {
    token: Option<String>,
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
struct ChartData {
    label: &'static str,
    series: Vec<NamedSeries>,
}

#[derive(Serialize)]
struct NamedSeries {
    name: &'static str,
    #[serde(flatten)]
    data: SeriesData,
}

#[derive(Serialize)]
struct MetricsMeta {
    source: &'static str,
    degraded: bool,
    sample_interval_seconds: u32,
    generated_at_epoch: u32,
    last_sample_epoch: u32,
    sample_failures_total: u64,
}

#[derive(Serialize)]
struct MetricsResponse {
    meta: MetricsMeta,
    charts: Vec<ChartData>,
}

fn series_data_from_points(points: &[(u32, f32)]) -> SeriesData {
    SeriesData {
        timestamps: points.iter().map(|(ts, _)| *ts).collect(),
        values: points.iter().map(|(_, v)| *v).collect(),
    }
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

fn range_seconds(range: Option<&String>) -> u32 {
    match range.map(String::as_str) {
        Some("1h") => 3600,
        Some("6h") => 21600,
        _ => 86400, // default 24h
    }
}

fn build_memory_charts(m: &MetricsStore, from: u32, now: u32) -> Vec<ChartData> {
    vec![
        ChartData {
            label: "Request Rate",
            series: vec![
                named("req/s", &m.ts_req_rate, from, now),
                named("4xx/s", &m.ts_4xx_rate, from, now),
                named("5xx/s", &m.ts_5xx_rate, from, now),
            ],
        },
        ChartData {
            label: "p95 Latency (ms)",
            series: vec![
                named("auth", &m.ts_p95_auth, from, now),
                named("profiles", &m.ts_p95_profiles, from, now),
                named("events", &m.ts_p95_events, from, now),
                named("uploads", &m.ts_p95_uploads, from, now),
                named("search", &m.ts_p95_search, from, now),
                named("matching", &m.ts_p95_matching, from, now),
                named("matrix", &m.ts_p95_matrix, from, now),
            ],
        },
        ChartData {
            label: "DB Pool",
            series: vec![
                named("size", &m.ts_pool_size, from, now),
                named("available", &m.ts_pool_available, from, now),
                named("waiting", &m.ts_pool_waiting, from, now),
            ],
        },
        ChartData {
            label: "DB Conn p95 (ms)",
            series: vec![named("conn_acq", &m.ts_p95_db_conn, from, now)],
        },
        ChartData {
            label: "Outbox Queue",
            series: vec![
                named("pending", &m.ts_outbox_pending, from, now),
                named("ready", &m.ts_outbox_ready, from, now),
                named("retrying", &m.ts_outbox_retrying, from, now),
                named("inflight", &m.ts_outbox_inflight, from, now),
                named("failed", &m.ts_outbox_failed, from, now),
            ],
        },
        ChartData {
            label: "Auth Cache",
            series: vec![
                named("hit%", &m.ts_auth_hit_rate, from, now),
                named("entries", &m.ts_auth_cache_entries, from, now),
            ],
        },
        ChartData {
            label: "Matrix Delivery",
            series: vec![
                named("room_ok", &m.ts_matrix_room_ok, from, now),
                named("room_fail", &m.ts_matrix_room_fail, from, now),
                named("membership_ok", &m.ts_matrix_membership_ok, from, now),
                named("membership_fail", &m.ts_matrix_membership_fail, from, now),
                named("avatar_ok", &m.ts_matrix_avatar_ok, from, now),
                named("avatar_fail", &m.ts_matrix_avatar_fail, from, now),
            ],
        },
        ChartData {
            label: "SMTP Delivery",
            series: vec![
                named("otp_ok", &m.ts_smtp_ok, from, now),
                named("otp_fail", &m.ts_smtp_fail, from, now),
                named("p95_ms", &m.ts_smtp_p95, from, now),
            ],
        },
    ]
}

// ── DB query helpers ──────────────────────────────────────────────────

async fn query_db(range_secs: u32) -> Option<(Vec<ScalarSample>, Vec<HistogramSample>)> {
    use crate::db::schema::{metrics_histogram_samples, metrics_scalar_samples};
    use chrono::{Duration as ChronoDuration, Utc};
    use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
    use diesel_async::RunQueryDsl;

    let mut conn = crate::db::conn().await.ok()?;
    let since = Utc::now() - ChronoDuration::seconds(i64::from(range_secs));

    let scalars: Vec<ScalarSample> = metrics_scalar_samples::table
        .filter(metrics_scalar_samples::ts.ge(since))
        .order(metrics_scalar_samples::ts.asc())
        .select(ScalarSample::as_select())
        .load(&mut *conn)
        .await
        .ok()?;

    let histograms: Vec<HistogramSample> = metrics_histogram_samples::table
        .filter(metrics_histogram_samples::ts.ge(since))
        .order(metrics_histogram_samples::ts.asc())
        .select(HistogramSample::as_select())
        .load(&mut *conn)
        .await
        .ok()?;

    Some((scalars, histograms))
}

/// Group scalar rows by (chart, series), summing values across instances at each timestamp.
fn aggregate_scalar_rows(rows: &[ScalarSample]) -> HashMap<(i16, i16), Vec<(u32, f32)>> {
    let mut by_key: HashMap<(i16, i16, u32), f32> = HashMap::new();
    for r in rows {
        let ts_epoch = r.ts.timestamp() as u32;
        *by_key.entry((r.chart, r.series, ts_epoch)).or_default() += r.value;
    }

    let mut result: HashMap<(i16, i16), Vec<(u32, f32)>> = HashMap::new();
    for ((chart, series, ts), value) in by_key {
        result.entry((chart, series)).or_default().push((ts, value));
    }
    for v in result.values_mut() {
        v.sort_by_key(|(ts, _)| *ts);
    }
    result
}

/// Merge histogram buckets across instances, keyed by (chart, series) → Vec<(ts, buckets)>.
fn aggregate_histogram_rows(
    rows: &[HistogramSample],
) -> HashMap<(i16, i16), Vec<(u32, [u64; HISTOGRAM_BUCKETS])>> {
    // First merge buckets for each (chart, series, ts) tuple
    let mut by_key: HashMap<(i16, i16, u32), [u64; HISTOGRAM_BUCKETS]> = HashMap::new();
    for r in rows {
        let ts_epoch = r.ts.timestamp() as u32;
        let buckets = by_key
            .entry((r.chart, r.series, ts_epoch))
            .or_insert([0u64; HISTOGRAM_BUCKETS]);
        if let Some(b) = buckets.get_mut(r.bucket as usize) {
            *b += r.count as u64;
        }
    }

    // Regroup into per-(chart, series) timelines
    let mut result: HashMap<(i16, i16), Vec<(u32, [u64; HISTOGRAM_BUCKETS])>> = HashMap::new();
    for ((chart, series, ts), buckets) in by_key {
        result
            .entry((chart, series))
            .or_default()
            .push((ts, buckets));
    }
    for v in result.values_mut() {
        v.sort_by_key(|(ts, _)| *ts);
    }
    result
}

fn series_from_aggregated(
    scalars: &HashMap<(i16, i16), Vec<(u32, f32)>>,
    chart: i16,
    series: i16,
    name: &'static str,
) -> NamedSeries {
    let empty = Vec::new();
    let data = scalars.get(&(chart, series)).unwrap_or(&empty);
    NamedSeries {
        name,
        data: series_data_from_points(data),
    }
}

/// Build a rate series from raw counts: divide each value by the sample interval.
fn series_as_rate(
    scalars: &HashMap<(i16, i16), Vec<(u32, f32)>>,
    chart: i16,
    series: i16,
    name: &'static str,
) -> NamedSeries {
    let empty = Vec::new();
    let data = scalars.get(&(chart, series)).unwrap_or(&empty);
    let points: Vec<(u32, f32)> = data
        .iter()
        .map(|(ts, v)| (*ts, v / SAMPLE_INTERVAL_SECS as f32))
        .collect();
    NamedSeries {
        name,
        data: series_data_from_points(&points),
    }
}

/// Build a hit-rate% series from raw hits (series 0) and misses (series 1).
fn series_hit_rate(scalars: &HashMap<(i16, i16), Vec<(u32, f32)>>, chart: i16) -> NamedSeries {
    let empty = Vec::new();
    let hits = scalars.get(&(chart, 0)).unwrap_or(&empty);
    let misses = scalars.get(&(chart, 1)).unwrap_or(&empty);

    let mut by_ts: HashMap<u32, (f32, f32)> = HashMap::new();
    for (ts, v) in hits {
        by_ts.entry(*ts).or_default().0 += v;
    }
    for (ts, v) in misses {
        by_ts.entry(*ts).or_default().1 += v;
    }

    let mut points: Vec<(u32, f32)> = by_ts
        .into_iter()
        .map(|(ts, (h, m))| {
            let total = h + m;
            let rate = if total > 0.0 {
                (h / total) * 100.0
            } else {
                0.0
            };
            (ts, rate)
        })
        .collect();
    points.sort_by_key(|(ts, _)| *ts);

    NamedSeries {
        name: "hit%",
        data: series_data_from_points(&points),
    }
}

/// Build a p95 series from pre-grouped histogram data (direct key lookup, no scan).
fn series_histogram_p95(
    histograms: &HashMap<(i16, i16), Vec<(u32, [u64; HISTOGRAM_BUCKETS])>>,
    chart: i16,
    series: i16,
    name: &'static str,
) -> NamedSeries {
    let empty = Vec::new();
    let data = histograms.get(&(chart, series)).unwrap_or(&empty);
    let points: Vec<(u32, f32)> = data
        .iter()
        .map(|(ts, buckets)| {
            let total: u64 = buckets.iter().sum();
            let snap = LatencyHistogramSnapshot {
                buckets: *buckets,
                overflow: 0,
                total,
            };
            (*ts, p95_from_snapshot(&snap))
        })
        .collect();

    NamedSeries {
        name,
        data: series_data_from_points(&points),
    }
}

fn build_timescaledb_charts(
    scalar_rows: &[ScalarSample],
    histogram_rows: &[HistogramSample],
) -> Vec<ChartData> {
    let scalars = aggregate_scalar_rows(scalar_rows);
    let histograms = aggregate_histogram_rows(histogram_rows);

    vec![
        ChartData {
            label: "Request Rate",
            series: vec![
                series_as_rate(&scalars, 0, 0, "req/s"),
                series_as_rate(&scalars, 0, 1, "4xx/s"),
                series_as_rate(&scalars, 0, 2, "5xx/s"),
            ],
        },
        ChartData {
            label: "p95 Latency (ms)",
            series: vec![
                series_histogram_p95(&histograms, 1, 0, "auth"),
                series_histogram_p95(&histograms, 1, 1, "profiles"),
                series_histogram_p95(&histograms, 1, 2, "events"),
                series_histogram_p95(&histograms, 1, 3, "uploads"),
                series_histogram_p95(&histograms, 1, 4, "search"),
                series_histogram_p95(&histograms, 1, 5, "matching"),
                series_histogram_p95(&histograms, 1, 6, "matrix"),
            ],
        },
        ChartData {
            label: "DB Pool",
            series: vec![
                series_from_aggregated(&scalars, 2, 0, "size"),
                series_from_aggregated(&scalars, 2, 1, "available"),
                series_from_aggregated(&scalars, 2, 2, "waiting"),
            ],
        },
        ChartData {
            label: "DB Conn p95 (ms)",
            series: vec![series_histogram_p95(&histograms, 3, 0, "conn_acq")],
        },
        ChartData {
            label: "Outbox Queue",
            series: vec![
                series_from_aggregated(&scalars, 4, 0, "pending"),
                series_from_aggregated(&scalars, 4, 1, "ready"),
                series_from_aggregated(&scalars, 4, 2, "retrying"),
                series_from_aggregated(&scalars, 4, 3, "inflight"),
                series_from_aggregated(&scalars, 4, 4, "failed"),
            ],
        },
        ChartData {
            label: "Auth Cache",
            series: vec![
                series_hit_rate(&scalars, 5),
                series_from_aggregated(&scalars, 5, 2, "entries"),
            ],
        },
        ChartData {
            label: "Matrix Delivery",
            series: vec![
                series_from_aggregated(&scalars, 6, 0, "room_ok"),
                series_from_aggregated(&scalars, 6, 1, "room_fail"),
                series_from_aggregated(&scalars, 6, 2, "membership_ok"),
                series_from_aggregated(&scalars, 6, 3, "membership_fail"),
                series_from_aggregated(&scalars, 6, 4, "avatar_ok"),
                series_from_aggregated(&scalars, 6, 5, "avatar_fail"),
            ],
        },
        ChartData {
            label: "SMTP Delivery",
            series: vec![
                series_from_aggregated(&scalars, 7, 0, "otp_ok"),
                series_from_aggregated(&scalars, 7, 1, "otp_fail"),
                series_histogram_p95(&histograms, 7, 2, "p95_ms"),
            ],
        },
    ]
}

// ── Handlers ──────────────────────────────────────────────────────────

fn meta_for(m: &MetricsStore, source: &'static str, degraded: bool) -> MetricsMeta {
    use std::sync::atomic::Ordering::Relaxed;
    let last_epoch = m.last_sample_epoch.load(Relaxed);
    let failures = m.sample_failures_total.load(Relaxed);
    MetricsMeta {
        source,
        degraded,
        sample_interval_seconds: SAMPLE_INTERVAL_SECS,
        generated_at_epoch: super::now_epoch(),
        last_sample_epoch: last_epoch,
        sample_failures_total: failures,
    }
}

async fn metrics_handler(headers: HeaderMap, Query(query): Query<MetricsQuery>) -> Response {
    if !check_ops_token(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let Some(m) = super::metrics() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let range = range_seconds(query.range.as_ref());

    // Try DB first, fall back to in-memory
    if let Some((scalar_rows, histogram_rows)) = query_db(range).await {
        let charts = build_timescaledb_charts(&scalar_rows, &histogram_rows);
        return Json(MetricsResponse {
            meta: meta_for(m, "timescaledb", false),
            charts,
        })
        .into_response();
    }

    // Fallback to in-memory ring buffers
    let now = super::now_epoch();
    let from = now.saturating_sub(range);
    let charts = build_memory_charts(m, from, now);

    Json(MetricsResponse {
        meta: meta_for(m, "memory", true),
        charts,
    })
    .into_response()
}

async fn dashboard_handler(Query(query): Query<DashboardQuery>) -> Response {
    if env_non_empty("OPS_STATUS_TOKEN").is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }

    match query.token.as_deref() {
        Some(t) if check_ops_token_value(t) => Html(DASHBOARD_HTML).into_response(),
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/v1/metrics/", get(dashboard_handler))
        .route("/api/v1/metrics", get(metrics_handler))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_scalar(
        ts_epoch: i64,
        instance_id: &str,
        chart: i16,
        series: i16,
        value: f32,
    ) -> ScalarSample {
        ScalarSample {
            ts: Utc.timestamp_opt(ts_epoch, 0).unwrap(),
            instance_id: instance_id.into(),
            chart,
            series,
            value,
        }
    }

    #[test]
    fn scalar_aggregation_across_instances() {
        let rows = vec![
            make_scalar(1000, "a", 0, 0, 10.0),
            make_scalar(1000, "b", 0, 0, 5.0),
            make_scalar(1010, "a", 0, 0, 20.0),
        ];
        let agg = aggregate_scalar_rows(&rows);
        let series = agg.get(&(0, 0)).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0], (1000, 15.0));
        assert_eq!(series[1], (1010, 20.0));
    }

    #[test]
    fn histogram_aggregation() {
        let rows = vec![
            HistogramSample {
                ts: Utc.timestamp_opt(1000, 0).unwrap(),
                instance_id: "a".into(),
                chart: 1,
                series: 0,
                bucket: 0,
                count: 50,
            },
            HistogramSample {
                ts: Utc.timestamp_opt(1000, 0).unwrap(),
                instance_id: "b".into(),
                chart: 1,
                series: 0,
                bucket: 0,
                count: 30,
            },
            HistogramSample {
                ts: Utc.timestamp_opt(1000, 0).unwrap(),
                instance_id: "a".into(),
                chart: 1,
                series: 0,
                bucket: 3,
                count: 5,
            },
        ];
        let agg = aggregate_histogram_rows(&rows);
        let timeline = agg.get(&(1, 0)).unwrap();
        assert_eq!(timeline.len(), 1);
        let (_, buckets) = &timeline[0];
        assert_eq!(buckets[0], 80); // 50+30
        assert_eq!(buckets[3], 5);
        assert_eq!(buckets[1], 0);
    }

    #[test]
    fn derived_hit_rate() {
        let rows = vec![
            make_scalar(1000, "a", 5, 0, 80.0), // hits
            make_scalar(1000, "a", 5, 1, 20.0), // misses
        ];
        let agg = aggregate_scalar_rows(&rows);
        let series = series_hit_rate(&agg, 5);
        assert_eq!(series.data.values.len(), 1);
        assert!((series.data.values[0] - 80.0).abs() < 0.01);
    }

    #[test]
    fn derived_hit_rate_zero_total() {
        let rows = vec![
            make_scalar(1000, "a", 5, 0, 0.0),
            make_scalar(1000, "a", 5, 1, 0.0),
        ];
        let agg = aggregate_scalar_rows(&rows);
        let series = series_hit_rate(&agg, 5);
        assert_eq!(series.data.values.len(), 1);
        assert!((series.data.values[0] - 0.0).abs() < 0.01);
    }

    #[test]
    fn timescaledb_charts_preserve_expected_series_shapes() {
        let scalar_rows = vec![
            make_scalar(1000, "a", 0, 0, SAMPLE_INTERVAL_SECS as f32 * 2.0),
            make_scalar(1000, "b", 0, 0, SAMPLE_INTERVAL_SECS as f32),
            make_scalar(1000, "a", 5, 0, 3.0),
            make_scalar(1000, "a", 5, 1, 1.0),
            make_scalar(1000, "a", 5, 2, 7.0),
        ];
        let histogram_rows = vec![HistogramSample {
            ts: Utc.timestamp_opt(1000, 0).unwrap(),
            instance_id: "a".into(),
            chart: 1,
            series: 0,
            bucket: 3,
            count: 10,
        }];

        let charts = build_timescaledb_charts(&scalar_rows, &histogram_rows);

        let request_rate = charts
            .iter()
            .find(|chart| chart.label == "Request Rate")
            .unwrap();
        let req_series = request_rate
            .series
            .iter()
            .find(|series| series.name == "req/s")
            .unwrap();
        assert_eq!(req_series.data.timestamps, vec![1000]);
        assert!((req_series.data.values[0] - 3.0).abs() < 0.01);

        let auth_cache = charts
            .iter()
            .find(|chart| chart.label == "Auth Cache")
            .unwrap();
        let hit_rate = auth_cache
            .series
            .iter()
            .find(|series| series.name == "hit%")
            .unwrap();
        let entries = auth_cache
            .series
            .iter()
            .find(|series| series.name == "entries")
            .unwrap();
        assert!((hit_rate.data.values[0] - 75.0).abs() < 0.01);
        assert_eq!(entries.data.values, vec![7.0]);

        let latency = charts
            .iter()
            .find(|chart| chart.label == "p95 Latency (ms)")
            .unwrap();
        let auth_latency = latency
            .series
            .iter()
            .find(|series| series.name == "auth")
            .unwrap();
        assert_eq!(auth_latency.data.timestamps, vec![1000]);
        assert_eq!(auth_latency.data.values.len(), 1);
        assert!(auth_latency.data.values[0] >= 0.0);
    }
}
