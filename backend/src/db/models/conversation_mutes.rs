use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::conversation_mutes;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = conversation_mutes)]
pub struct ConversationMute {
    pub user_id: i32,
    pub conversation_id: Uuid,
    pub muted_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversation_mutes)]
pub struct NewConversationMute {
    pub user_id: i32,
    pub conversation_id: Uuid,
    pub muted_at: DateTime<Utc>,
}
