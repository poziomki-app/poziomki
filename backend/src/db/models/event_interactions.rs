use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::event_interactions;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = event_interactions)]
#[diesel(primary_key(profile_id, event_id, kind))]
pub struct EventInteraction {
    pub profile_id: Uuid,
    pub event_id: Uuid,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
