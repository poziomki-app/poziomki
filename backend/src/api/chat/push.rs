use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db;
use crate::db::schema::push_subscriptions;

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
/// Only the conversation ID is sent through ntfy — no message content, sender
/// names, or avatar URLs. The client uses this as a wake-up signal and fetches
/// actual message data through the authenticated WebSocket/API.
pub async fn notify_push(user_ids: Vec<i32>, conversation_id: Uuid, _sender_id: i32, _body: &str) {
    // Resolve ntfy topics for target users
    let topics = match resolve_ntfy_topics(&user_ids).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "failed to resolve push topics");
            return;
        }
    };

    let client = push_client();
    let ntfy_token = crate::api::common::env_non_empty("NTFY_TOKEN");

    let push_data = serde_json::json!({
        "room_id": conversation_id.to_string(),
    });

    for (ntfy_topic, ntfy_server) in &topics {
        let topic_prefix: String = ntfy_topic.chars().take(8).collect();
        let url = format!("{ntfy_server}/{ntfy_topic}");
        let mut req = client
            .post(&url)
            .header("Title", "new_message")
            .json(&push_data);
        if let Some(ref token) = ntfy_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let result = req.send().await;

        match result {
            Ok(resp) => match resp.error_for_status() {
                Ok(_) => {
                    tracing::info!(topic = topic_prefix, "push_delivered");
                }
                Err(e) => {
                    tracing::warn!(topic = topic_prefix, error = %e, "push notification rejected");
                }
            },
            Err(e) => {
                tracing::warn!(topic = topic_prefix, error = %e, "push notification failed");
            }
        }
    }
}

async fn resolve_ntfy_topics(
    user_ids: &[i32],
) -> Result<Vec<(String, String)>, crate::error::AppError> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }

    let ntfy_server = resolved_ntfy_server();

    // Push delivery runs after a message/event commit, decoupled from the
    // originating viewer transaction. Use an anon tx — the only column read
    // is `ntfy_topic`, which is not sensitive (it's the destination the user
    // registered with us). Once push_subscriptions ships Tier-A RLS this
    // will move to a narrow SECURITY DEFINER helper.
    let owned_ids = user_ids.to_vec();
    let topics: Vec<String> = db::with_anon_tx(move |conn| {
        async move {
            push_subscriptions::table
                .filter(push_subscriptions::user_id.eq_any(&owned_ids))
                .select(push_subscriptions::ntfy_topic)
                .load(conn)
                .await
        }
        .scope_boxed()
    })
    .await?;

    Ok(topics
        .into_iter()
        .map(|topic| (topic, ntfy_server.clone()))
        .collect())
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
