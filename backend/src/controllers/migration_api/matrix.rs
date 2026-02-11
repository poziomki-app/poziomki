use axum::{extract::State, http::HeaderMap, Json};
use loco_rs::{app::AppContext, prelude::*};
use serde::Deserialize;

pub(super) use super::matrix_support;
use super::{error_response, state::require_auth_db, ErrorSpec};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixSessionRequest {
    #[serde(default)]
    device_name: Option<String>,
}

pub(super) async fn create_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<MatrixSessionRequest>,
) -> Result<Response> {
    let user_pid = {
        let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };
        user.pid.to_string()
    };

    match do_create_session(&user_pid, payload.device_name.as_deref(), &headers).await {
        Ok(response) | Err(response) => Ok(response),
    }
}

async fn do_create_session(
    user_pid: &str,
    device_name: Option<&str>,
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

    let config = matrix_support::build_conn_config(user_pid, device_name);
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

    matrix_support::build_session_response(public_homeserver, matrix_auth, headers)
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
