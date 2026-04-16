use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::user_audit_log;

#[derive(Debug, Insertable)]
#[diesel(table_name = user_audit_log)]
pub struct NewUserAuditLog {
    pub id: Uuid,
    pub user_pid: Uuid,
    pub action: String,
    pub created_at: DateTime<Utc>,
}
