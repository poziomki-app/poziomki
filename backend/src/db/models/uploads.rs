use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::uploads;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = uploads)]
pub struct Upload {
    pub id: Uuid,
    pub filename: String,
    pub owner_id: Option<Uuid>,
    pub context: String,
    pub context_id: Option<String>,
    pub mime_type: String,
    pub deleted: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub has_variants: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = uploads)]
pub struct NewUpload {
    pub id: Uuid,
    pub filename: String,
    pub owner_id: Option<Uuid>,
    pub context: String,
    pub context_id: Option<String>,
    pub mime_type: String,
    pub deleted: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub has_variants: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, AsChangeset, Default)]
#[diesel(table_name = uploads)]
pub struct UploadChangeset {
    pub filename: Option<String>,
    pub owner_id: Option<Option<Uuid>>,
    pub context: Option<String>,
    pub context_id: Option<Option<String>>,
    pub mime_type: Option<String>,
    pub deleted: Option<bool>,
    pub thumbhash: Option<Option<Vec<u8>>>,
    pub has_variants: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}
