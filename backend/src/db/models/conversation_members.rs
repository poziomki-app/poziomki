use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::conversation_members;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = conversation_members, primary_key(conversation_id, user_id))]
pub struct ConversationMember {
    pub conversation_id: Uuid,
    pub user_id: i32,
    pub joined_at: DateTime<Utc>,
    pub last_read_message_id: Option<Uuid>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversation_members)]
pub struct NewConversationMember {
    pub conversation_id: Uuid,
    pub user_id: i32,
    pub joined_at: DateTime<Utc>,
}
