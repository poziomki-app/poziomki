use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::message_reads;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = message_reads, primary_key(message_id, user_id))]
pub struct MessageRead {
    pub message_id: Uuid,
    pub user_id: i32,
    pub read_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = message_reads)]
pub struct NewMessageRead {
    pub message_id: Uuid,
    pub user_id: i32,
    pub read_at: DateTime<Utc>,
}
