use uuid::Uuid;

use crate::api::chat::push_apns;
use crate::db;

const ALLOWED_NTFY_HOSTS: &[&str] = &["ntfy.poziomki.app"];
const DEFAULT_NTFY_SERVER: &str = "https://ntfy.poziomki.app";

fn is_allowed_ntfy_server(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    ALLOWED_NTFY_HOSTS.contains(&host)
}

/// Return the configured ntfy server if it passes the allowlist, otherwise the
/// safe default. A poisoned `NTFY_SERVER` env must not redirect push traffic to
/// an attacker-controlled host.
pub fn resolved_ntfy_server() -> String {
    match crate::api::common::env_non_empty("NTFY_SERVER") {
        Some(configured) if is_allowed_ntfy_server(&configured) => configured,
        Some(configured) => {
            tracing::warn!(
                configured = %configured,
                "NTFY_SERVER rejected by allowlist; falling back to default"
            );
            DEFAULT_NTFY_SERVER.to_string()
        }
        None => DEFAULT_NTFY_SERVER.to_string(),
    }
}

fn push_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "push client builder failed, using default");
                reqwest::Client::new()
            })
    })
}

/// Send push notifications to conversation members for a new message.
///
/// Only the conversation ID is sent — no message content, sender names, or
/// avatar URLs. The client uses this as a wake-up signal and fetches actual
/// message data through the authenticated WebSocket/API. Android subscribers
/// are notified via ntfy; iOS subscribers via APNs.
pub async fn notify_push(user_ids: Vec<i32>, conversation_id: Uuid, _sender_id: i32, _body: &str) {
    let subs = match resolve_subscriptions(&user_ids).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "failed to resolve push subscriptions");
            return;
        }
    };

    let ntfy_server = resolved_ntfy_server();
    let client = push_client();
    let ntfy_token = crate::api::common::env_non_empty("NTFY_TOKEN");
    let push_data = serde_json::json!({
        "room_id": conversation_id.to_string(),
    });

    for sub in subs {
        match sub.platform.as_str() {
            "android" => {
                let Some(ntfy_topic) = sub.ntfy_topic else {
                    continue;
                };
                send_ntfy(
                    client,
                    &ntfy_server,
                    &ntfy_topic,
                    ntfy_token.as_deref(),
                    &push_data,
                )
                .await;
            }
            "ios" => {
                let Some(apns_token) = sub.apns_token else {
                    continue;
                };
                if let Err(e) = push_apns::send_apns(
                    &apns_token,
                    conversation_id,
                    "Nowa wiadomość",
                    "Masz nową wiadomość w Poziomki",
                )
                .await
                {
                    tracing::warn!(error = %e, "apns send failed");
                }
            }
            other => {
                tracing::warn!(platform = %other, "unknown push platform; skipping");
            }
        }
    }
}

async fn send_ntfy(
    client: &reqwest::Client,
    ntfy_server: &str,
    ntfy_topic: &str,
    ntfy_token: Option<&str>,
    payload: &serde_json::Value,
) {
    let topic_prefix: String = ntfy_topic.chars().take(8).collect();
    let url = format!("{ntfy_server}/{ntfy_topic}");
    let mut req = client
        .post(&url)
        .header("Title", "new_message")
        .json(payload);
    if let Some(token) = ntfy_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    match req.send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(_) => tracing::info!(topic = topic_prefix, "push_delivered"),
            Err(e) => {
                tracing::warn!(topic = topic_prefix, error = %e, "push notification rejected");
            }
        },
        Err(e) => tracing::warn!(topic = topic_prefix, error = %e, "push notification failed"),
    }
}

async fn resolve_subscriptions(
    user_ids: &[i32],
) -> Result<Vec<db::PushSubscriptionRow>, crate::error::AppError> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    // Narrow SECURITY DEFINER helper: returns only platform + tokens for the
    // given user ids. Server-side delivery only.
    let mut conn = crate::db::conn().await?;
    let subs = db::push_subscriptions_for_users(&mut conn, user_ids).await?;
    Ok(subs)
}

#[cfg(test)]
mod tests {
    use super::is_allowed_ntfy_server;

    #[test]
    fn accepts_allowlisted_host() {
        assert!(is_allowed_ntfy_server("https://ntfy.poziomki.app"));
        assert!(is_allowed_ntfy_server("https://ntfy.poziomki.app/"));
        assert!(is_allowed_ntfy_server("https://ntfy.poziomki.app/topic"));
    }

    #[test]
    fn rejects_non_https() {
        assert!(!is_allowed_ntfy_server("http://ntfy.poziomki.app"));
    }

    #[test]
    fn rejects_other_hosts() {
        assert!(!is_allowed_ntfy_server("https://evil.example.com"));
        assert!(!is_allowed_ntfy_server(
            "https://ntfy.poziomki.app.evil.com"
        ));
        assert!(!is_allowed_ntfy_server(
            "https://evil.com/ntfy.poziomki.app"
        ));
    }

    #[test]
    fn rejects_userinfo_smuggling() {
        assert!(!is_allowed_ntfy_server(
            "https://ntfy.poziomki.app@evil.com"
        ));
    }
}
