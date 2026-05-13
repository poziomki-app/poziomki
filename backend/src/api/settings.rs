type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::DataResponse;
use crate::api::auth_or_respond;
use crate::db::models::user_settings::{NewUserSetting, UserSetting, UserSettingChangeset};
use crate::db::schema::user_settings;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub(in crate::api) struct UserSettingsResponse {
    pub(in crate::api) theme: String,
    pub(in crate::api) language: String,
    #[serde(rename = "notificationsEnabled")]
    pub(in crate::api) notifications_enabled: bool,
    #[serde(rename = "privacyShowProgram")]
    pub(in crate::api) privacy_show_program: bool,
    #[serde(rename = "privacyDiscoverable")]
    pub(in crate::api) privacy_discoverable: bool,
    #[serde(rename = "notifyDms")]
    pub(in crate::api) notify_dms: bool,
    #[serde(rename = "notifyEventChats")]
    pub(in crate::api) notify_event_chats: bool,
    #[serde(rename = "notifyTagEvents")]
    pub(in crate::api) notify_tag_events: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub(in crate::api) struct UpdateSettingsBody {
    #[serde(default)]
    pub(in crate::api) theme: Option<String>,
    #[serde(default)]
    pub(in crate::api) language: Option<String>,
    #[serde(default)]
    pub(in crate::api) notifications_enabled: Option<bool>,
    #[serde(default)]
    pub(in crate::api) privacy_show_program: Option<bool>,
    #[serde(default)]
    pub(in crate::api) privacy_discoverable: Option<bool>,
    #[serde(default)]
    pub(in crate::api) notify_dms: Option<bool>,
    #[serde(default)]
    pub(in crate::api) notify_event_chats: Option<bool>,
    #[serde(default)]
    pub(in crate::api) notify_tag_events: Option<bool>,
}

fn model_to_response(model: &UserSetting) -> UserSettingsResponse {
    UserSettingsResponse {
        theme: model.theme.clone(),
        language: model.language.clone(),
        notifications_enabled: model.notifications_enabled,
        privacy_show_program: model.privacy_show_program,
        privacy_discoverable: model.privacy_discoverable,
        notify_dms: model.notify_dms,
        notify_event_chats: model.notify_event_chats,
        notify_tag_events: model.notify_tag_events,
    }
}

fn default_response() -> UserSettingsResponse {
    UserSettingsResponse {
        theme: "system".to_string(),
        language: "system".to_string(),
        notifications_enabled: true,
        privacy_show_program: true,
        privacy_discoverable: true,
        notify_dms: true,
        notify_event_chats: true,
        notify_tag_events: true,
    }
}

pub(super) async fn settings_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let mut conn = crate::db::conn().await?;
    let settings = user_settings::table
        .filter(user_settings::user_id.eq(user.id))
        .first::<UserSetting>(&mut conn)
        .await
        .optional()?;

    let data = settings
        .as_ref()
        .map_or_else(default_response, model_to_response);
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn settings_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<UpdateSettingsBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let mut conn = crate::db::conn().await?;
    let existing = user_settings::table
        .filter(user_settings::user_id.eq(user.id))
        .first::<UserSetting>(&mut conn)
        .await
        .optional()?;

    let updated = if let Some(record) = existing {
        let changeset = UserSettingChangeset {
            theme: body.theme.clone(),
            language: body.language.clone(),
            notifications_enabled: body.notifications_enabled,
            privacy_show_program: body.privacy_show_program,
            privacy_discoverable: body.privacy_discoverable,
            notify_dms: body.notify_dms,
            notify_event_chats: body.notify_event_chats,
            notify_tag_events: body.notify_tag_events,
            updated_at: Some(Utc::now()),
        };
        diesel::update(user_settings::table.find(record.id))
            .set(&changeset)
            .get_result::<UserSetting>(&mut conn)
            .await?
    } else {
        let now = Utc::now();
        let new = NewUserSetting {
            id: Uuid::new_v4(),
            user_id: user.id,
            theme: body.theme.clone().unwrap_or_else(|| "system".to_string()),
            language: body
                .language
                .clone()
                .unwrap_or_else(|| "system".to_string()),
            notifications_enabled: body.notifications_enabled.unwrap_or(true),
            privacy_show_program: body.privacy_show_program.unwrap_or(true),
            privacy_discoverable: body.privacy_discoverable.unwrap_or(true),
            notify_dms: body.notify_dms.unwrap_or(true),
            notify_event_chats: body.notify_event_chats.unwrap_or(true),
            notify_tag_events: body.notify_tag_events.unwrap_or(true),
            created_at: now,
            updated_at: now,
        };
        diesel::insert_into(user_settings::table)
            .values(&new)
            .get_result::<UserSetting>(&mut conn)
            .await?
    };

    let data = model_to_response(&updated);
    Ok(Json(DataResponse { data }).into_response())
}
