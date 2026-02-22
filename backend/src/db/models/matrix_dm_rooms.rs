use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::matrix_dm_rooms;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = matrix_dm_rooms)]
pub struct MatrixDmRoom {
    pub id: Uuid,
    pub user_low_pid: Uuid,
    pub user_high_pid: Uuid,
    pub room_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = matrix_dm_rooms)]
pub struct NewMatrixDmRoom {
    pub id: Uuid,
    pub user_low_pid: Uuid,
    pub user_high_pid: Uuid,
    pub room_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
