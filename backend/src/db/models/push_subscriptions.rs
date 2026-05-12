use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::push_subscriptions;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = push_subscriptions)]
pub struct PushSubscription {
    pub id: Uuid,
    pub user_id: i32,
    pub device_id: String,
    pub fcm_token: String,
    pub created_at: DateTime<Utc>,
    pub platform: String,
    pub token_updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = push_subscriptions)]
pub struct NewPushSubscription {
    pub id: Uuid,
    pub user_id: i32,
    pub device_id: String,
    pub fcm_token: String,
    pub created_at: DateTime<Utc>,
    pub platform: String,
    pub token_updated_at: DateTime<Utc>,
}
