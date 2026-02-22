use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::profile_tags;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = profile_tags)]
#[diesel(primary_key(profile_id, tag_id))]
pub struct ProfileTag {
    pub profile_id: Uuid,
    pub tag_id: Uuid,
}
