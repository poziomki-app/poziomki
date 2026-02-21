use std::time::Duration;

use axum::{extract::State, http::HeaderMap, Json};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::super::state::require_auth_db;
use super::{chat_bootstrap_error, matrix_support, MatrixSessionRequest};
use crate::models::_entities::profiles;

pub(super) async fn create_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<MatrixSessionRequest>,
) -> Result<Response> {
    let (user_pid, user_name, profile_picture) = {
        let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };
        let pic = profiles::Entity::find()
            .filter(profiles::Column::UserId.eq(user.id))
            .one(&ctx.db)
            .await
            .ok()
            .flatten()
            .and_then(|p| p.profile_picture);
        (user.pid.to_string(), user.name, pic)
    };

    match do_create_session(
        &user_pid,
        &user_name,
        payload.device_name.as_deref(),
        payload.device_id.as_deref(),
        profile_picture.as_deref(),
        &headers,
    )
    .await
    {
        Ok(response) | Err(response) => Ok(response),
    }
}

async fn do_create_session(
    user_pid: &str,
    user_name: &str,
    device_name: Option<&str>,
    device_id: Option<&str>,
    profile_picture_filename: Option<&str>,
    headers: &HeaderMap,
) -> std::result::Result<Response, Response> {
    let internal_homeserver = matrix_support::resolve_homeserver().ok_or_else(|| {
        chat_bootstrap_error(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            "Messaging service is not configured",
            "CHAT_NOT_CONFIGURED",
        )
    })?;

    let public_homeserver =
        matrix_support::resolve_public_homeserver().unwrap_or_else(|| internal_homeserver.clone());

    let config =
        matrix_support::build_conn_config(user_pid, device_name, device_id).map_err(|error| {
            tracing::warn!(%error, "matrix session bootstrap is not configured");
            chat_bootstrap_error(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                headers,
                "Messaging service is not configured",
                "CHAT_NOT_CONFIGURED",
            )
        })?;
    let http_client = matrix_support::init_http_client(headers)?;

    let matrix_auth = matrix_support::try_matrix_auth(&http_client, &internal_homeserver, &config)
        .await
        .map_err(|error| {
            tracing::warn!(
                status_code = error.status_code,
                errcode = error.errcode,
                message = error.message,
                "matrix session bootstrap failed"
            );
            chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
            )
        })?;

    if let Err(error) = matrix_support::set_display_name(
        &http_client,
        &internal_homeserver,
        &matrix_auth.access_token,
        &matrix_auth.user_id,
        user_name,
    )
    .await
    {
        tracing::warn!(error = %error, "failed to set matrix display name");
    }

    if let Some(pic_filename) = profile_picture_filename {
        if let Err(error) = sync_matrix_avatar(
            &http_client,
            &internal_homeserver,
            &matrix_auth.access_token,
            &matrix_auth.user_id,
            pic_filename,
        )
        .await
        {
            tracing::warn!(error = %error, "failed to set matrix avatar");
        }
    }

    matrix_support::build_session_response(public_homeserver, matrix_auth, headers)
}

async fn sync_matrix_avatar(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    user_id: &str,
    pic_filename: &str,
) -> std::result::Result<(), String> {
    let filename = super::super::extract_filename(pic_filename);
    let content_type = matrix_support::content_type_from_filename(&filename)
        .ok_or_else(|| format!("unsupported image type: {filename}"))?;
    let bytes = super::super::uploads::uploads_storage::read(&filename)
        .await
        .map_err(|e| format!("failed to read {filename} from storage: {e:?}"))?;
    let mxc_uri = matrix_support::upload_media(
        http_client,
        homeserver,
        access_token,
        bytes,
        content_type,
        &filename,
    )
    .await?;
    matrix_support::set_avatar_url(http_client, homeserver, access_token, user_id, &mxc_uri).await
}

pub(super) async fn sync_profile_avatar_best_effort(
    user_pid: &Uuid,
    profile_picture_filename: Option<&str>,
) {
    let Some(pic_filename) = profile_picture_filename else {
        return;
    };
    let Some(internal_homeserver) = resolve_avatar_sync_homeserver() else {
        return;
    };
    let Some(config) = build_avatar_sync_config(user_pid) else {
        return;
    };
    let Some(http_client) = build_avatar_sync_http_client() else {
        return;
    };
    let Some(matrix_auth) =
        authenticate_avatar_sync(&http_client, &internal_homeserver, &config).await
    else {
        return;
    };

    if let Err(error) = sync_matrix_avatar(
        &http_client,
        &internal_homeserver,
        &matrix_auth.access_token,
        &matrix_auth.user_id,
        pic_filename,
    )
    .await
    {
        tracing::warn!(%error, "failed to sync matrix avatar after profile update");
    }
}

fn resolve_avatar_sync_homeserver() -> Option<String> {
    matrix_support::resolve_homeserver()
}

fn build_avatar_sync_config(user_pid: &Uuid) -> Option<matrix_support::MatrixConnConfig> {
    match matrix_support::build_conn_config(&user_pid.to_string(), None, None) {
        Ok(config) => Some(config),
        Err(error) => {
            tracing::warn!(%error, "matrix avatar sync skipped: matrix bootstrap is not configured");
            None
        }
    }
}

fn build_avatar_sync_http_client() -> Option<reqwest::Client> {
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => Some(client),
        Err(error) => {
            tracing::warn!(error = %error, "matrix avatar sync skipped: failed to build http client");
            None
        }
    }
}

async fn authenticate_avatar_sync(
    http_client: &reqwest::Client,
    homeserver: &str,
    config: &matrix_support::MatrixConnConfig,
) -> Option<matrix_support::MatrixAuthResponse> {
    match matrix_support::try_matrix_auth(http_client, homeserver, config).await {
        Ok(auth) => Some(auth),
        Err(error) => {
            tracing::warn!(
                status_code = error.status_code,
                errcode = error.errcode,
                message = error.message,
                "matrix avatar sync skipped: failed to authenticate"
            );
            None
        }
    }
}
