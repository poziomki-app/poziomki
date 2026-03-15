use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::messages;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, QueryableByName)]
#[diesel(table_name = messages)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: i32,
    pub body: String,
    pub kind: String,
    pub attachment_upload_id: Option<Uuid>,
    pub reply_to_id: Option<Uuid>,
    pub client_id: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: i32,
    pub body: String,
    pub kind: String,
    pub attachment_upload_id: Option<Uuid>,
    pub reply_to_id: Option<Uuid>,
    pub client_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
