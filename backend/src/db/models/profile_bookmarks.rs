use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::profile_bookmarks;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = profile_bookmarks)]
#[diesel(primary_key(profile_id, target_profile_id))]
pub struct ProfileBookmark {
    pub profile_id: Uuid,
    pub target_profile_id: Uuid,
    pub created_at: DateTime<Utc>,
}
