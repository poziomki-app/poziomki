use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::message_reactions;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = message_reactions)]
pub struct MessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: i32,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = message_reactions)]
pub struct NewMessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: i32,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}
