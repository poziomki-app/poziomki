use std::time::Duration;

type Result<T> = crate::error::AppResult<T>;

use crate::api::auth_or_respond;
use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, Json};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{chat_bootstrap_error, matrix_service, MatrixSessionRequest};
use crate::db::models::profiles::Profile;
use crate::db::schema::profiles;

pub(super) async fn create_session(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<MatrixSessionRequest>,
) -> Result<Response> {
    let (user_pid, user_name, profile_picture) = {
        let (_session, user) = auth_or_respond!(headers);
        let pic = {
            let mut conn = crate::db::conn().await.ok();
            match conn.as_mut() {
                Some(conn) => profiles::table
                    .filter(profiles::user_id.eq(user.id))
                    .first::<Profile>(conn)
                    .await
                    .ok()
                    .and_then(|p| p.profile_picture),
                None => None,
            }
        };
        (user.pid.to_string(), user.name, pic)
    };

    let request = SessionRequest {
        user_pid: &user_pid,
        user_name: &user_name,
        device_name: payload.device_name.as_deref(),
        device_id: payload.device_id.as_deref(),
        profile_picture_filename: profile_picture.as_deref(),
    };
    match do_create_session(&request, &headers).await {
        Ok(response) | Err(response) => Ok(response),
    }
}

struct SessionRequest<'a> {
    user_pid: &'a str,
    user_name: &'a str,
    device_name: Option<&'a str>,
    device_id: Option<&'a str>,
    profile_picture_filename: Option<&'a str>,
}

async fn do_create_session(
    req: &SessionRequest<'_>,
    headers: &HeaderMap,
) -> std::result::Result<Response, Response> {
    let internal_homeserver = matrix_service::resolve_homeserver().ok_or_else(|| {
        chat_bootstrap_error(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            "Messaging service is not configured",
            "CHAT_NOT_CONFIGURED",
        )
    })?;

    let public_homeserver =
        matrix_service::resolve_public_homeserver().unwrap_or_else(|| internal_homeserver.clone());

    let config = matrix_service::build_conn_config(req.user_pid, req.device_name, req.device_id)
        .map_err(|error| {
            tracing::warn!(%error, "matrix session bootstrap is not configured");
            chat_bootstrap_error(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                headers,
                "Messaging service is not configured",
                "CHAT_NOT_CONFIGURED",
            )
        })?;
    let http_client = matrix_service::init_http_client(headers)?;

    let matrix_auth = matrix_service::try_matrix_auth(&http_client, &internal_homeserver, &config)
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

    let client = matrix_service::MatrixClient::new(
        &http_client,
        &internal_homeserver,
        &matrix_auth.access_token,
    );

    if let Err(error) = client
        .set_display_name(&matrix_auth.user_id, req.user_name)
        .await
    {
        tracing::warn!(error = %error, "failed to set matrix display name");
    }

    if let Some(pic_filename) = req.profile_picture_filename {
        let avatar_result = sync_matrix_avatar(&client, &matrix_auth.user_id, pic_filename).await;
        if let Err(error) = avatar_result {
            tracing::warn!(error = %error, "failed to set matrix avatar");
        }
    }

    matrix_service::build_session_response(public_homeserver, matrix_auth, headers)
}

async fn sync_matrix_avatar(
    client: &matrix_service::MatrixClient<'_>,
    user_id: &str,
    pic_filename: &str,
) -> std::result::Result<(), String> {
    let filename = super::super::extract_filename(pic_filename);
    let content_type = matrix_service::content_type_from_filename(&filename)
        .ok_or_else(|| format!("unsupported image type: {filename}"))?;
    let bytes = super::super::uploads::uploads_storage::read(&filename)
        .await
        .map_err(|e| format!("failed to read {filename} from storage: {e:?}"))?;
    let mxc_uri = client.upload_media(bytes, content_type, &filename).await?;
    client.set_avatar_url(user_id, &mxc_uri).await
}

pub(super) async fn sync_profile_avatar_best_effort(
    user_pid: &uuid::Uuid,
    profile_picture_filename: Option<&str>,
) {
    if let Some(pic_filename) = profile_picture_filename {
        let result = try_sync_profile_avatar(user_pid, pic_filename).await;
        if let Err(error) = result {
            tracing::warn!(%error, "failed to sync matrix avatar after profile update");
        }
    }
}

async fn try_sync_profile_avatar(
    user_pid: &uuid::Uuid,
    pic_filename: &str,
) -> std::result::Result<(), String> {
    let internal_homeserver =
        resolve_avatar_sync_homeserver().ok_or("matrix homeserver not configured")?;
    let config = build_avatar_sync_config(user_pid).ok_or("matrix bootstrap not configured")?;
    let http_client = build_avatar_sync_http_client().ok_or("failed to build http client")?;
    let matrix_auth = authenticate_avatar_sync(&http_client, &internal_homeserver, &config)
        .await
        .ok_or("failed to authenticate for avatar sync")?;

    let client = matrix_service::MatrixClient::new(
        &http_client,
        &internal_homeserver,
        &matrix_auth.access_token,
    );
    sync_matrix_avatar(&client, &matrix_auth.user_id, pic_filename).await
}

fn resolve_avatar_sync_homeserver() -> Option<String> {
    matrix_service::resolve_homeserver()
}

fn build_avatar_sync_config(user_pid: &uuid::Uuid) -> Option<matrix_service::MatrixConnConfig> {
    match matrix_service::build_conn_config(&user_pid.to_string(), None, None) {
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
    config: &matrix_service::MatrixConnConfig,
) -> Option<matrix_service::MatrixAuthResponse> {
    match matrix_service::try_matrix_auth(http_client, homeserver, config).await {
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
