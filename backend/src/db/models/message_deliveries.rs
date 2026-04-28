use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::message_deliveries;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = message_deliveries, primary_key(message_id, user_id))]
pub struct MessageDelivery {
    pub message_id: Uuid,
    pub user_id: i32,
    pub delivered_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = message_deliveries)]
pub struct NewMessageDelivery {
    pub message_id: Uuid,
    pub user_id: i32,
    pub delivered_at: DateTime<Utc>,
}
