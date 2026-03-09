use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::tags;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = tags)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub scope: String,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub parent_id: Option<Uuid>,
    pub onboarding_order: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tags)]
pub struct NewTag {
    pub id: Uuid,
    pub name: String,
    pub scope: String,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub parent_id: Option<Uuid>,
    pub onboarding_order: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
