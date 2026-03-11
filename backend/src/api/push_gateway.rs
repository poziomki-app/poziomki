use std::{collections::HashSet, time::Duration};

use axum::response::Response;
use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use url::Url;

type Result<T> = crate::error::AppResult<T>;

#[derive(Debug, Deserialize)]
pub(in crate::api) struct MatrixPushRequest {
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
pub(in crate::api) struct PushGatewayAuthQuery {
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

fn query_token_fallback_enabled() -> bool {
    std::env::var("PUSH_GATEWAY_QUERY_TOKEN_FALLBACK")
        .ok()
        .is_none_or(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

fn provided_gateway_token(headers: &HeaderMap, query: &PushGatewayAuthQuery) -> Option<String> {
    bearer_token(headers).or_else(|| {
        if query_token_fallback_enabled() {
            query.token.clone()
        } else {
            None
        }
    })
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

fn validated_pushkey_host(pushkey: &str) -> Option<(Url, String)> {
    let url = Url::parse(pushkey).ok()?;
    let host = url.host_str().map(str::to_ascii_lowercase)?;
    Some((url, host))
}

fn is_allowed_pushkey(pushkey: &str, allowed_hosts: &HashSet<String>) -> bool {
    let Some((url, host)) = validated_pushkey_host(pushkey) else {
        return false;
    };
    allowed_hosts.contains(&host)
        && (url.scheme() == "https" || (url.scheme() == "http" && is_loopback_host(&host)))
}

fn push_target_host(pushkey: &str) -> String {
    Url::parse(pushkey)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "invalid".to_string())
}

fn gateway_auth_error(headers: &HeaderMap, query: &PushGatewayAuthQuery) -> Option<Response> {
    let Some(expected_token) = push_gateway_token() else {
        tracing::error!("PUSH_GATEWAY_TOKEN is not configured");
        return Some(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Push gateway is not configured",
        ));
    };
    if !token_matches(&expected_token, provided_gateway_token(headers, query)) {
        return Some(error_response(StatusCode::UNAUTHORIZED, "Unauthorized"));
    }
    None
}

fn configured_allowed_hosts() -> Option<HashSet<String>> {
    let allowed_hosts = allowed_push_hosts();
    if allowed_hosts.is_empty() {
        tracing::error!("No push target allowlist configured");
        return None;
    }
    Some(allowed_hosts)
}

fn build_push_http_client() -> std::result::Result<reqwest::Client, crate::error::AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| crate::error::AppError::Any(error.into()))
}

fn notification_title(notification: &PushNotification) -> &str {
    notification
        .sender_display_name
        .as_deref()
        .or(notification.sender.as_deref())
        .unwrap_or("New message")
}

fn notification_body_raw(notification: &PushNotification) -> String {
    serde_json::json!({
        "event_id": notification.event_id,
        "room_id": notification.room_id,
        "sender": notification.sender,
    })
    .to_string()
}

struct DeliveryContext<'a> {
    http_client: &'a reqwest::Client,
    allowed_hosts: &'a HashSet<String>,
    title: &'a str,
    body_raw: &'a str,
}

async fn deliver_notification_to_device(
    ctx: &DeliveryContext<'_>,
    notification: &PushNotification,
    device: &PushDevice,
) -> bool {
    let target_host = push_target_host(&device.pushkey);
    if reject_unallowed_pushkey(&device.pushkey, &target_host, ctx.allowed_hosts) {
        return false;
    }

    let response =
        match send_device_notification(ctx.http_client, &device.pushkey, ctx.title, ctx.body_raw)
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(
                    pushkey_host = %target_host,
                    error = %err,
                    "failed to deliver push notification to ntfy"
                );
                return false;
            }
        };

    log_delivery_response(&target_host, notification, &response)
}

fn reject_unallowed_pushkey(
    pushkey: &str,
    target_host: &str,
    allowed_hosts: &HashSet<String>,
) -> bool {
    if is_allowed_pushkey(pushkey, allowed_hosts) {
        return false;
    }
    tracing::warn!(
        pushkey_host = %target_host,
        "push notification rejected: pushkey host not allowed"
    );
    true
}

async fn send_device_notification(
    http_client: &reqwest::Client,
    pushkey: &str,
    title: &str,
    body_raw: &str,
) -> std::result::Result<reqwest::Response, reqwest::Error> {
    http_client
        .post(pushkey)
        .header("Title", title)
        .header("Priority", "4")
        .header("Tags", "speech_balloon")
        .body(body_raw.to_string())
        .send()
        .await
}

fn log_delivery_response(
    target_host: &str,
    notification: &PushNotification,
    response: &reqwest::Response,
) -> bool {
    if response.status().is_success() {
        tracing::debug!(
            pushkey_host = %target_host,
            event_id = ?notification.event_id,
            "push notification delivered to ntfy"
        );
        return true;
    }

    tracing::warn!(
        pushkey_host = %target_host,
        status = %response.status(),
        "ntfy rejected push notification"
    );
    false
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
    if let Some(response) = gateway_auth_error(&headers, &query) {
        return Ok(response);
    }
    let Some(allowed_hosts) = configured_allowed_hosts() else {
        return Ok(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Push gateway is not configured",
        ));
    };

    let notification = &payload.notification;
    let http_client = build_push_http_client()?;
    let mut rejected = Vec::new();
    let title = notification_title(notification);
    let body_raw = notification_body_raw(notification);

    let ctx = DeliveryContext {
        http_client: &http_client,
        allowed_hosts: &allowed_hosts,
        title,
        body_raw: &body_raw,
    };

    for device in &notification.devices {
        let delivered = deliver_notification_to_device(&ctx, notification, device).await;
        if !delivered {
            rejected.push(device.pushkey.clone());
        }
    }

    Ok(Json(PushGatewayResponse { rejected }).into_response())
}
