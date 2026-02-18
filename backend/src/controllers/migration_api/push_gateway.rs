use std::{collections::HashSet, time::Duration};

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use url::Url;

#[derive(Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct MatrixPushRequest {
    notification: PushNotification,
}

#[derive(Debug, Deserialize)]
struct PushNotification {
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    room_id: Option<String>,
    #[serde(default)]
    sender: Option<String>,
    #[serde(default)]
    sender_display_name: Option<String>,
    #[serde(default)]
    devices: Vec<PushDevice>,
}

#[derive(Debug, Deserialize)]
struct PushDevice {
    #[allow(dead_code)]
    app_id: String,
    pushkey: String,
}

#[derive(Debug, Serialize)]
struct PushGatewayResponse {
    rejected: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct PushGatewayAuthQuery {
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct PushGatewayErrorResponse {
    error: String,
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(PushGatewayErrorResponse {
            error: message.to_string(),
        }),
    )
        .into_response()
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn push_gateway_token() -> Option<String> {
    env_non_empty("PUSH_GATEWAY_TOKEN")
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?.to_str().ok()?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") {
        Some(token.trim().to_string())
    } else {
        None
    }
}

fn provided_gateway_token(headers: &HeaderMap, query: &PushGatewayAuthQuery) -> Option<String> {
    query.token.clone().or_else(|| bearer_token(headers))
}

fn token_matches(expected: &str, provided: Option<String>) -> bool {
    provided.is_some_and(|value| {
        value.len() == expected.len() && bool::from(value.as_bytes().ct_eq(expected.as_bytes()))
    })
}

fn parse_allowed_hosts(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn allowed_push_hosts() -> HashSet<String> {
    if let Some(raw) = env_non_empty("PUSH_GATEWAY_ALLOWED_HOSTS") {
        return parse_allowed_hosts(&raw);
    }

    env_non_empty("NTFY_SERVER_URL")
        .and_then(|url| Url::parse(&url).ok())
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .into_iter()
        .collect()
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn is_allowed_pushkey(pushkey: &str, allowed_hosts: &HashSet<String>) -> bool {
    let Ok(url) = Url::parse(pushkey) else {
        return false;
    };
    let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
        return false;
    };
    if !allowed_hosts.contains(&host) {
        return false;
    }

    matches!(url.scheme(), "https") || (url.scheme() == "http" && is_loopback_host(&host))
}

fn push_target_host(pushkey: &str) -> String {
    Url::parse(pushkey)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "invalid".to_string())
}

/// Matrix push gateway endpoint: `POST /_matrix/push/v1/notify`
///
/// Called by the homeserver (Tuwunel) when a user has a registered pusher.
/// Forwards the push data to the device's ntfy topic URL (the pushkey).
pub(super) async fn notify(
    headers: HeaderMap,
    Query(query): Query<PushGatewayAuthQuery>,
    Json(payload): Json<MatrixPushRequest>,
) -> Result<Response> {
    let Some(expected_token) = push_gateway_token() else {
        tracing::error!("PUSH_GATEWAY_TOKEN is not configured");
        return Ok(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Push gateway is not configured",
        ));
    };

    if !token_matches(&expected_token, provided_gateway_token(&headers, &query)) {
        return Ok(error_response(StatusCode::UNAUTHORIZED, "Unauthorized"));
    }

    let allowed_hosts = allowed_push_hosts();
    if allowed_hosts.is_empty() {
        tracing::error!("No push target allowlist configured");
        return Ok(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Push gateway is not configured",
        ));
    }

    let notification = &payload.notification;
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| loco_rs::Error::Any(error.into()))?;
    let mut rejected = Vec::new();

    let title = notification
        .sender_display_name
        .as_deref()
        .or(notification.sender.as_deref())
        .unwrap_or("New message");

    let body = serde_json::json!({
        "event_id": notification.event_id,
        "room_id": notification.room_id,
        "sender": notification.sender,
    });
    let body_raw = body.to_string();

    for device in &notification.devices {
        let target_host = push_target_host(&device.pushkey);
        if !is_allowed_pushkey(&device.pushkey, &allowed_hosts) {
            tracing::warn!(
                pushkey_host = %target_host,
                "push notification rejected: pushkey host not allowed"
            );
            rejected.push(device.pushkey.clone());
            continue;
        }

        let result = http_client
            .post(&device.pushkey)
            .header("Title", title)
            .header("Priority", "4")
            .header("Tags", "speech_balloon")
            .body(body_raw.clone())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!(
                    pushkey_host = %target_host,
                    event_id = ?notification.event_id,
                    "push notification delivered to ntfy"
                );
            }
            Ok(resp) => {
                tracing::warn!(
                    pushkey_host = %target_host,
                    status = %resp.status(),
                    "ntfy rejected push notification"
                );
                rejected.push(device.pushkey.clone());
            }
            Err(err) => {
                tracing::warn!(
                    pushkey_host = %target_host,
                    error = %err,
                    "failed to deliver push notification to ntfy"
                );
                rejected.push(device.pushkey.clone());
            }
        }
    }

    Ok(Json(PushGatewayResponse { rejected }).into_response())
}
