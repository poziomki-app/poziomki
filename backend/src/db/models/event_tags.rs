use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::event_tags;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = event_tags)]
#[diesel(primary_key(event_id, tag_id))]
pub struct EventTag {
    pub event_id: Uuid,
    pub tag_id: Uuid,
}
