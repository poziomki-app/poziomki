use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::user_feedback;

#[derive(Debug, Insertable)]
#[diesel(table_name = user_feedback)]
pub struct NewUserFeedback {
    pub id: Uuid,
    pub user_id: i32,
    pub rating: i16,
    pub message: Option<String>,
    pub app_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub feature_request: Option<String>,
}
