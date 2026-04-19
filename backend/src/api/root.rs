use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::app::AppContext;

use super::env_non_empty;

type Result<T> = crate::error::AppResult<T>;

#[derive(Clone, Debug, Serialize)]
struct RootInfoResponse {
    docs: &'static str,
    message: &'static str,
    version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutboxStatusResponse {
    status: &'static str,
    metrics: crate::jobs::OutboxStatsSnapshot,
}

pub(super) async fn health() -> Result<Response> {
    Ok(Json(HealthResponse { status: "ok" }).into_response())
}

fn ops_status_token() -> Option<String> {
    env_non_empty("OPS_STATUS_TOKEN")
}

fn ops_token_matches(headers: &HeaderMap) -> bool {
    let Some(expected) = ops_status_token() else {
        return false;
    };
    let actual = headers
        .get("x-ops-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    // Constant-time compare so response timing doesn't leak bytes of
    // the expected token to an attacker probing /ops/*.
    expected.as_bytes().ct_eq(actual.as_bytes()).unwrap_u8() == 1
}

pub(super) async fn outbox_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if ops_status_token().is_none() {
        return Ok((axum::http::StatusCode::NOT_FOUND, "not found").into_response());
    }

    // Rate limit by caller IP before token compare — caps online
    // brute-force of OPS_STATUS_TOKEN.
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::OpsAuth,
    )
    .await
    {
        return Ok(*resp);
    }

    if !ops_token_matches(&headers) {
        return Ok((axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response());
    }

    let metrics = crate::jobs::outbox_stats_snapshot().await?;
    let status = if metrics.failed_jobs > 0 || metrics.oldest_ready_job_age_seconds > 60 {
        "degraded"
    } else {
        "ok"
    };

    Ok(Json(OutboxStatusResponse { status, metrics }).into_response())
}

pub(super) async fn root() -> Result<Response> {
    Ok(Json(RootInfoResponse {
        docs: "/api/docs",
        message: "poziomki API v1",
        version: "1.0.0",
    })
    .into_response())
}
