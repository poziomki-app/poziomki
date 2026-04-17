use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::users;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub pid: Uuid,
    pub email: String,
    pub password: String,
    pub api_key: String,
    pub name: String,
    pub reset_token: Option<String>,
    pub reset_sent_at: Option<DateTime<Utc>>,
    pub email_verification_token: Option<String>,
    pub email_verification_sent_at: Option<DateTime<Utc>>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub magic_link_token: Option<String>,
    pub magic_link_expiration: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_review_stub: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub pid: Uuid,
    pub email: String,
    pub password: String,
    pub api_key: String,
    pub name: String,
}

#[derive(Debug, AsChangeset, Default)]
#[diesel(table_name = users)]
pub struct UserChangeset {
    pub email_verified_at: Option<Option<DateTime<Utc>>>,
    pub updated_at: Option<DateTime<Utc>>,
}
