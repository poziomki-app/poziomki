use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::auth_rate_limits;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = auth_rate_limits)]
pub struct AuthRateLimit {
    pub id: Uuid,
    pub rate_key: String,
    pub window_start: DateTime<Utc>,
    pub attempts: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = auth_rate_limits)]
pub struct NewAuthRateLimit {
    pub id: Uuid,
    pub rate_key: String,
    pub window_start: DateTime<Utc>,
    pub attempts: i32,
    pub updated_at: DateTime<Utc>,
}
