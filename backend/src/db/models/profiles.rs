use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::profiles;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = profiles)]
pub struct Profile {
    pub id: Uuid,
    pub user_id: i32,
    pub name: String,
    pub bio: Option<String>,
    pub age: Option<i16>,
    pub profile_picture: Option<String>,
    pub images: Option<serde_json::Value>,
    pub program: Option<String>,
    pub gradient_start: Option<String>,
    pub gradient_end: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = profiles)]
pub struct NewProfile {
    pub id: Uuid,
    pub user_id: i32,
    pub name: String,
    pub bio: Option<String>,
    pub age: Option<i16>,
    pub profile_picture: Option<String>,
    pub images: Option<serde_json::Value>,
    pub program: Option<String>,
    pub gradient_start: Option<String>,
    pub gradient_end: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, AsChangeset, Default)]
#[diesel(table_name = profiles)]
pub struct ProfileChangeset {
    pub name: Option<String>,
    pub bio: Option<Option<String>>,
    pub age: Option<Option<i16>>,
    pub profile_picture: Option<Option<String>>,
    pub images: Option<Option<serde_json::Value>>,
    pub program: Option<Option<String>>,
    pub gradient_start: Option<Option<String>>,
    pub gradient_end: Option<Option<String>>,
    pub updated_at: Option<DateTime<Utc>>,
}
