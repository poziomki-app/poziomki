use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::event_attendees;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = event_attendees)]
#[diesel(primary_key(event_id, profile_id))]
pub struct EventAttendee {
    pub event_id: Uuid,
    pub profile_id: Uuid,
    pub status: String,
}
