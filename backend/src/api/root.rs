use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use url::Url;

use crate::app::AppContext;

use super::{env_non_empty, state::require_auth_db};

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

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigResponse {
    data: MatrixConfigData,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigData {
    homeserver: Option<String>,
    chat_mode: &'static str,
    push_gateway_url: Option<String>,
    ntfy_server: Option<String>,
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
    headers
        .get("x-ops-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| actual == expected)
}

pub(super) async fn outbox_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if ops_status_token().is_none() {
        return Ok((axum::http::StatusCode::NOT_FOUND, "not found").into_response());
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

pub(super) async fn matrix_config(headers: HeaderMap) -> Result<Response> {
    let authenticated = require_auth_db(&headers).await.is_ok();

    let homeserver = env_non_empty("MATRIX_HOMESERVER_PUBLIC_URL")
        .or_else(|| env_non_empty("MATRIX_HOMESERVER_URL"));
    let push_gateway_url = env_non_empty("PUSH_GATEWAY_URL").and_then(|url| {
        if authenticated {
            Some(url)
        } else {
            redact_push_gateway_url(&url)
        }
    });
    let ntfy_server = env_non_empty("NTFY_SERVER_URL");
    Ok(Json(MatrixConfigResponse {
        data: MatrixConfigData {
            homeserver,
            chat_mode: "matrix-native",
            push_gateway_url,
            ntfy_server,
        },
    })
    .into_response())
}

fn redact_push_gateway_url(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    Some(url.into())
}

#[cfg(test)]
mod tests {
    use super::redact_push_gateway_url;

    #[test]
    #[allow(clippy::expect_used)]
    fn redact_push_gateway_url_strips_query_and_fragment() {
        let raw = "https://user:pass@push.example/_matrix/push/v1/notify?token=secret#frag";
        let redacted = redact_push_gateway_url(raw).expect("valid url");
        assert_eq!(redacted, "https://push.example/_matrix/push/v1/notify");
    }
}
