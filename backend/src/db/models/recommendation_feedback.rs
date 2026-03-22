use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::recommendation_feedback;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = recommendation_feedback)]
#[diesel(primary_key(profile_id, event_id))]
pub struct RecommendationFeedback {
    pub profile_id: Uuid,
    pub event_id: Uuid,
    pub feedback: String,
    pub created_at: DateTime<Utc>,
}
