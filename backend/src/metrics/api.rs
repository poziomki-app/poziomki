use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

use super::store::TimeSeries;

fn ops_status_token() -> Option<String> {
    std::env::var("OPS_STATUS_TOKEN").ok().filter(|v| !v.trim().is_empty())
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
        .and_then(|v| v.to_str().ok())
        .is_some_and(|actual| token_matches(actual, &expected))
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
struct MetricsResponse {
    charts: Vec<ChartData>,
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

async fn metrics_handler(
    headers: HeaderMap,
    Query(query): Query<MetricsQuery>,
) -> Response {
    if !check_ops_token(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let Some(m) = super::metrics() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    let range = range_seconds(query.range.as_ref());
    let from = now.saturating_sub(range);

    let charts = vec![
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
    ];

    Json(MetricsResponse { charts }).into_response()
}

pub fn routes() -> Router {
    Router::new().route("/api/v1/metrics", get(metrics_handler))
}
