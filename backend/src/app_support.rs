use crate::error::AppResult;
use diesel_async::RunQueryDsl;

const TRUNCATE_ORDER: &[&str] = &[
    "job_outbox",
    "auth_rate_limits",
    "message_reactions",
    "messages",
    "conversation_members",
    "conversations",
    "push_subscriptions",
    "event_attendees",
    "event_interactions",
    "event_tags",
    "profile_tags",
    "events",
    "uploads",
    "user_settings",
    "otp_codes",
    "sessions",
    "profiles",
    "degrees",
    "tags",
    "users",
];

pub async fn truncate_all_tables() -> AppResult<()> {
    let mut conn = crate::db::conn().await?;

    for table in TRUNCATE_ORDER {
        diesel::sql_query(format!("DELETE FROM \"{table}\""))
            .execute(&mut conn)
            .await?;
    }
    Ok(())
}
