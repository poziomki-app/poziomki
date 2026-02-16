use axum::{response::IntoResponse, Json};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

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

/// Matrix push gateway endpoint: `POST /_matrix/push/v1/notify`
///
/// Called by the homeserver (Tuwunel) when a user has a registered pusher.
/// Forwards the push data to the device's ntfy topic URL (the pushkey).
pub(super) async fn notify(Json(payload): Json<MatrixPushRequest>) -> Result<Response> {
    let notification = &payload.notification;
    let http_client = reqwest::Client::new();
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

    for device in &notification.devices {
        let result = http_client
            .post(&device.pushkey)
            .header("Title", title)
            .header("Priority", "4")
            .header("Tags", "speech_balloon")
            .body(body.to_string())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!(
                    pushkey = %device.pushkey,
                    event_id = ?notification.event_id,
                    "push notification delivered to ntfy"
                );
            }
            Ok(resp) => {
                tracing::warn!(
                    pushkey = %device.pushkey,
                    status = %resp.status(),
                    "ntfy rejected push notification"
                );
                rejected.push(device.pushkey.clone());
            }
            Err(err) => {
                tracing::warn!(
                    pushkey = %device.pushkey,
                    error = %err,
                    "failed to deliver push notification to ntfy"
                );
                rejected.push(device.pushkey.clone());
            }
        }
    }

    Ok(Json(PushGatewayResponse { rejected }).into_response())
}
