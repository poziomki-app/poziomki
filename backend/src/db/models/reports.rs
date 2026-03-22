use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::reports;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = reports)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub reason: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = reports)]
pub struct NewReport {
    pub reporter_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub reason: String,
    pub description: Option<String>,
}
