use axum::{http::HeaderMap, response::IntoResponse, Json};
use loco_rs::prelude::*;

use super::state::{lock_state, require_auth, DataResponse};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub(in crate::controllers::migration_api) struct UserSettingsResponse {
    pub(in crate::controllers::migration_api) theme: String,
    pub(in crate::controllers::migration_api) language: String,
    #[serde(rename = "notificationsEnabled")]
    pub(in crate::controllers::migration_api) notifications_enabled: bool,
    #[serde(rename = "privacyShowAge")]
    pub(in crate::controllers::migration_api) privacy_show_age: bool,
    #[serde(rename = "privacyShowProgram")]
    pub(in crate::controllers::migration_api) privacy_show_program: bool,
    #[serde(rename = "privacyDiscoverable")]
    pub(in crate::controllers::migration_api) privacy_discoverable: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::migration_api) struct UpdateSettingsBody {
    #[serde(default)]
    pub(in crate::controllers::migration_api) theme: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) language: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) notifications_enabled: Option<bool>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) privacy_show_age: Option<bool>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) privacy_show_program: Option<bool>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) privacy_discoverable: Option<bool>,
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(in crate::controllers::migration_api) struct UserSettingsRecord {
    pub(in crate::controllers::migration_api) theme: String,
    pub(in crate::controllers::migration_api) language: String,
    pub(in crate::controllers::migration_api) notifications_enabled: bool,
    pub(in crate::controllers::migration_api) privacy_show_age: bool,
    pub(in crate::controllers::migration_api) privacy_show_program: bool,
    pub(in crate::controllers::migration_api) privacy_discoverable: bool,
}

impl UserSettingsRecord {
    fn new() -> Self {
        Self {
            theme: "system".to_string(),
            language: "system".to_string(),
            notifications_enabled: true,
            privacy_show_age: true,
            privacy_show_program: true,
            privacy_discoverable: true,
        }
    }

    fn apply(&mut self, patch: UpdateSettingsBody) {
        if let Some(v) = patch.theme {
            self.theme = v;
        }
        if let Some(v) = patch.language {
            self.language = v;
        }
        if let Some(v) = patch.notifications_enabled {
            self.notifications_enabled = v;
        }
        if let Some(v) = patch.privacy_show_age {
            self.privacy_show_age = v;
        }
        if let Some(v) = patch.privacy_show_program {
            self.privacy_show_program = v;
        }
        if let Some(v) = patch.privacy_discoverable {
            self.privacy_discoverable = v;
        }
    }
}

impl From<&UserSettingsRecord> for UserSettingsResponse {
    fn from(record: &UserSettingsRecord) -> Self {
        Self {
            theme: record.theme.clone(),
            language: record.language.clone(),
            notifications_enabled: record.notifications_enabled,
            privacy_show_age: record.privacy_show_age,
            privacy_show_program: record.privacy_show_program,
            privacy_discoverable: record.privacy_discoverable,
        }
    }
}

#[allow(clippy::significant_drop_tightening)]
pub(super) async fn settings_get(headers: HeaderMap) -> Result<Response> {
    let mut state = lock_state();
    let (_, user) = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let settings = state.user_settings.get(&user.id).map_or_else(
        || UserSettingsResponse::from(&UserSettingsRecord::new()),
        UserSettingsResponse::from,
    );

    Ok(Json(DataResponse { data: settings }).into_response())
}

#[allow(clippy::significant_drop_tightening)]
pub(super) async fn settings_update(
    headers: HeaderMap,
    Json(body): Json<UpdateSettingsBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let (_, user) = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let settings = state
        .user_settings
        .entry(user.id)
        .or_insert_with(UserSettingsRecord::new);
    settings.apply(body);

    let response = UserSettingsResponse::from(&*settings);
    Ok(Json(DataResponse { data: response }).into_response())
}
