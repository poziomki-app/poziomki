use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::schema::push_subscriptions;

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

    let ntfy_server = crate::api::common::env_non_empty("NTFY_SERVER")
        .unwrap_or_else(|| "https://ntfy.poziomki.app".to_string());

    let mut conn = crate::db::conn().await?;
    let topics: Vec<String> = push_subscriptions::table
        .filter(push_subscriptions::user_id.eq_any(user_ids))
        .select(push_subscriptions::ntfy_topic)
        .load(&mut conn)
        .await?;

    Ok(topics
        .into_iter()
        .map(|topic| (topic, ntfy_server.clone()))
        .collect())
}
