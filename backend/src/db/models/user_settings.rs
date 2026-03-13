use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::schema::user_settings;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_settings)]
#[allow(clippy::struct_excessive_bools)]
pub struct UserSetting {
    pub id: Uuid,
    pub user_id: i32,
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
    pub privacy_show_program: bool,
    pub privacy_discoverable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_settings)]
#[allow(clippy::struct_excessive_bools)]
pub struct NewUserSetting {
    pub id: Uuid,
    pub user_id: i32,
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
    pub privacy_show_program: bool,
    pub privacy_discoverable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = user_settings)]
#[allow(clippy::struct_excessive_bools)]
pub struct UserSettingChangeset {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub notifications_enabled: Option<bool>,
    pub privacy_show_program: Option<bool>,
    pub privacy_discoverable: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}
