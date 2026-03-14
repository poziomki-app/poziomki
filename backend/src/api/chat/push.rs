use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::schema::{profiles, push_subscriptions, users};

/// Send push notifications to offline users for a new message.
pub async fn notify_offline(
    offline_user_ids: Vec<i32>,
    conversation_id: Uuid,
    sender_id: i32,
    body: &str,
) {
    // Resolve sender name + avatar
    let Some((sender_name, sender_avatar)) = resolve_sender_profile(sender_id).await else {
        return;
    };

    // Resolve ntfy topics for offline users
    let topics = match resolve_ntfy_topics(&offline_user_ids).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "failed to resolve push topics");
            return;
        }
    };

    let client = reqwest::Client::new();

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
        let result = client
            .post(&url)
            .header("Title", &sender_name)
            .body(push_data.to_string())
            .send()
            .await;

        if let Err(e) = result {
            tracing::warn!(
                topic = ntfy_topic,
                error = %e,
                "push notification failed"
            );
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
