use axum::{extract::State, http::HeaderMap, Json};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::EntityTrait;
use serde::Deserialize;
use std::time::Duration;

pub(super) use super::matrix_support;
use super::{error_response, state::require_auth_db, ErrorSpec};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixSessionRequest {
    #[serde(default)]
    device_name: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
}

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
        let pic = {
            use crate::models::_entities::profiles;
            profiles::Entity::find()
                .filter(profiles::Column::UserId.eq(user.id))
                .one(&ctx.db)
                .await
                .ok()
                .flatten()
                .and_then(|p| p.profile_picture)
        };
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

    // Set the user's Matrix display name (best-effort, don't fail the session)
    if let Err(e) = matrix_support::set_display_name(
        &http_client,
        &internal_homeserver,
        &matrix_auth.access_token,
        &matrix_auth.user_id,
        user_name,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to set matrix display name");
    }

    // Set the user's Matrix avatar (best-effort, don't fail the session)
    if let Some(pic_filename) = profile_picture_filename {
        if let Err(e) = sync_matrix_avatar(
            &http_client,
            &internal_homeserver,
            &matrix_auth.access_token,
            &matrix_auth.user_id,
            pic_filename,
        )
        .await
        {
            tracing::warn!(error = %e, "failed to set matrix avatar");
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
    let filename = super::extract_filename(pic_filename);
    let content_type = matrix_support::content_type_from_filename(&filename)
        .ok_or_else(|| format!("unsupported image type: {filename}"))?;
    let bytes = super::uploads::uploads_storage::read(&filename)
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
    user_pid: &uuid::Uuid,
    profile_picture_filename: Option<&str>,
) {
    let Some(pic_filename) = profile_picture_filename else {
        return;
    };

    let Some(internal_homeserver) = matrix_support::resolve_homeserver() else {
        return;
    };

    let config = match matrix_support::build_conn_config(&user_pid.to_string(), None, None) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(%error, "matrix avatar sync skipped: matrix bootstrap is not configured");
            return;
        }
    };

    let http_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(error = %error, "matrix avatar sync skipped: failed to build http client");
            return;
        }
    };

    let matrix_auth =
        match matrix_support::try_matrix_auth(&http_client, &internal_homeserver, &config).await {
            Ok(auth) => auth,
            Err(error) => {
                tracing::warn!(
                    status_code = error.status_code,
                    errcode = error.errcode,
                    message = error.message,
                    "matrix avatar sync skipped: failed to authenticate"
                );
                return;
            }
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

pub(super) fn chat_bootstrap_error(
    status: axum::http::StatusCode,
    headers: &HeaderMap,
    message: &str,
    code: &'static str,
) -> Response {
    error_response(
        status,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code,
            details: None,
        },
    )
}
