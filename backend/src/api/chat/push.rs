use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::schema::{profiles, push_subscriptions, users};

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
pub async fn notify_push(user_ids: Vec<i32>, conversation_id: Uuid, sender_id: i32, body: &str) {
    // Resolve sender name + avatar
    let Some((sender_name, sender_avatar)) = resolve_sender_profile(sender_id).await else {
        return;
    };

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

    // Truncate body for push
    let push_body = if body.chars().count() > 200 {
        let truncated: String = body.chars().take(197).collect();
        format!("{truncated}...")
    } else {
        body.to_string()
    };

    let avatar_url = sender_avatar
        .as_ref()
        .and_then(|filename| crate::api::imgproxy_signing::signed_url(filename, "thumb", "webp"));

    let push_data = serde_json::json!({
        "room_id": conversation_id.to_string(),
        "sender": sender_name,
        "body": push_body,
        "avatar": avatar_url,
    });

    for (ntfy_topic, ntfy_server) in &topics {
        let url = format!("{ntfy_server}/{ntfy_topic}");
        let mut req = client
            .post(&url)
            .header("Title", &sender_name)
            .json(&push_data);
        if let Some(ref token) = ntfy_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let result = req.send().await;

        match result {
            Ok(resp) => {
                if let Err(e) = resp.error_for_status() {
                    tracing::warn!(topic = ntfy_topic, error = %e, "push notification rejected");
                }
            }
            Err(e) => {
                tracing::warn!(topic = ntfy_topic, error = %e, "push notification failed");
            }
        }
    }
}

async fn resolve_sender_profile(sender_id: i32) -> Option<(String, Option<String>)> {
    let mut conn = crate::db::conn().await.ok()?;
    profiles::table
        .inner_join(users::table.on(users::id.eq(profiles::user_id)))
        .filter(users::id.eq(sender_id))
        .select((profiles::name, profiles::profile_picture))
        .first::<(String, Option<String>)>(&mut conn)
        .await
        .ok()
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
