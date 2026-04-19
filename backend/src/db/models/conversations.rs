use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::conversations;

#[derive(Debug, Clone, Queryable, QueryableByName, Selectable, Identifiable)]
#[diesel(table_name = conversations)]
pub struct Conversation {
    pub id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub event_id: Option<Uuid>,
    pub user_low_id: Option<i32>,
    pub user_high_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversations)]
pub struct NewConversation {
    pub id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub event_id: Option<Uuid>,
    pub user_low_id: Option<i32>,
    pub user_high_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
