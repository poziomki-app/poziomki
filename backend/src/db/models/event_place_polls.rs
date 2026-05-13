use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::{event_place_options, event_place_polls, event_place_votes};

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = event_place_polls)]
pub struct EventPlacePoll {
    pub id: Uuid,
    pub event_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = event_place_polls)]
pub struct NewEventPlacePoll {
    pub id: Uuid,
    pub event_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = event_place_options)]
pub struct EventPlaceOption {
    pub id: Uuid,
    pub poll_id: Uuid,
    pub label: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = event_place_options)]
pub struct NewEventPlaceOption {
    pub id: Uuid,
    pub poll_id: Uuid,
    pub label: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = event_place_votes)]
#[diesel(primary_key(poll_id, profile_id))]
pub struct EventPlaceVote {
    pub poll_id: Uuid,
    pub profile_id: Uuid,
    pub option_id: Uuid,
    pub voted_at: DateTime<Utc>,
}
