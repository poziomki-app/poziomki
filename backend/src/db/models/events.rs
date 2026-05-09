use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::events;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = events)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub category: Option<String>,
    pub location: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub creator_id: Uuid,
    pub conversation_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub max_attendees: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub requires_approval: bool,
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub category: Option<String>,
    pub location: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub creator_id: Uuid,
    pub conversation_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub max_attendees: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub requires_approval: bool,
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, AsChangeset, Default)]
#[diesel(table_name = events)]
pub struct EventChangeset {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub cover_image: Option<Option<String>>,
    pub category: Option<Option<String>>,
    pub location: Option<Option<String>>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<Option<DateTime<Utc>>>,
    pub conversation_id: Option<Option<String>>,
    pub latitude: Option<Option<f64>>,
    pub longitude: Option<Option<f64>>,
    pub max_attendees: Option<Option<i32>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub requires_approval: Option<bool>,
    pub recurrence_rule: Option<Option<String>>,
}
