use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::otp_codes;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = otp_codes)]
pub struct OtpCode {
    pub id: Uuid,
    pub email: String,
    pub code: String,
    pub attempts: i16,
    pub expires_at: DateTime<Utc>,
    pub last_sent_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = otp_codes)]
pub struct NewOtpCode {
    pub id: Uuid,
    pub email: String,
    pub code: String,
    pub attempts: i16,
    pub expires_at: DateTime<Utc>,
    pub last_sent_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
