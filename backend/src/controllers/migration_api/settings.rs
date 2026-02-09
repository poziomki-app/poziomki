use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use super::state::{require_auth_db, DataResponse};
use crate::models::_entities::user_settings;

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

fn model_to_response(model: &user_settings::Model) -> UserSettingsResponse {
    UserSettingsResponse {
        theme: model.theme.clone(),
        language: model.language.clone(),
        notifications_enabled: model.notifications_enabled,
        privacy_show_age: model.privacy_show_age,
        privacy_show_program: model.privacy_show_program,
        privacy_discoverable: model.privacy_discoverable,
    }
}

fn default_response() -> UserSettingsResponse {
    UserSettingsResponse {
        theme: "system".to_string(),
        language: "system".to_string(),
        notifications_enabled: true,
        privacy_show_age: true,
        privacy_show_program: true,
        privacy_discoverable: true,
    }
}

pub(super) async fn settings_get(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let settings = user_settings::Entity::find()
        .filter(user_settings::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let data = settings
        .as_ref()
        .map_or_else(default_response, model_to_response);
    Ok(Json(DataResponse { data }).into_response())
}

fn apply_settings_update(
    existing: user_settings::Model,
    body: &UpdateSettingsBody,
) -> user_settings::ActiveModel {
    let mut active: user_settings::ActiveModel = existing.into();
    if let Some(v) = &body.theme {
        active.theme = ActiveValue::Set(v.clone());
    }
    if let Some(v) = &body.language {
        active.language = ActiveValue::Set(v.clone());
    }
    if let Some(v) = body.notifications_enabled {
        active.notifications_enabled = ActiveValue::Set(v);
    }
    if let Some(v) = body.privacy_show_age {
        active.privacy_show_age = ActiveValue::Set(v);
    }
    if let Some(v) = body.privacy_show_program {
        active.privacy_show_program = ActiveValue::Set(v);
    }
    if let Some(v) = body.privacy_discoverable {
        active.privacy_discoverable = ActiveValue::Set(v);
    }
    active.updated_at = ActiveValue::Set(Utc::now().into());
    active
}

fn create_new_settings(user_id: i32, body: &UpdateSettingsBody) -> user_settings::ActiveModel {
    let now = Utc::now();
    user_settings::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user_id),
        theme: ActiveValue::Set(body.theme.clone().unwrap_or_else(|| "system".to_string())),
        language: ActiveValue::Set(
            body.language
                .clone()
                .unwrap_or_else(|| "system".to_string()),
        ),
        notifications_enabled: ActiveValue::Set(body.notifications_enabled.unwrap_or(true)),
        privacy_show_age: ActiveValue::Set(body.privacy_show_age.unwrap_or(true)),
        privacy_show_program: ActiveValue::Set(body.privacy_show_program.unwrap_or(true)),
        privacy_discoverable: ActiveValue::Set(body.privacy_discoverable.unwrap_or(true)),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    }
}

pub(super) async fn settings_update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<UpdateSettingsBody>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let existing = user_settings::Entity::find()
        .filter(user_settings::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let updated = if let Some(record) = existing {
        let active = apply_settings_update(record, &body);
        active
            .update(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    } else {
        let new_settings = create_new_settings(user.id, &body);
        new_settings
            .insert(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    };

    let data = model_to_response(&updated);
    Ok(Json(DataResponse { data }).into_response())
}
