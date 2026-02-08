use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use super::{
    error_response,
    state::{lock_state, require_auth},
    ErrorSpec,
};

const DEFAULT_DEVICE_NAME: &str = "Poziomki Mobile";
const DEFAULT_PASSWORD_PEPPER: &str = "poziomki-dev-matrix-pepper";
const DEV_REGISTRATION_TOKEN: &str = "poziomki-dev-token";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixSessionRequest {
    #[serde(default)]
    device_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixSessionEnvelope {
    data: MatrixSessionData,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatrixSessionData {
    homeserver: String,
    access_token: String,
    refresh_token: String,
    user_id: String,
    device_id: String,
    expires_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct MatrixAuthResponse {
    access_token: String,
    user_id: String,
    device_id: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in_ms: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct MatrixErrorBody {
    #[serde(default)]
    errcode: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct MatrixRequestError {
    status_code: u16,
    errcode: Option<String>,
    message: String,
}

impl MatrixRequestError {
    fn is_user_in_use(&self) -> bool {
        self.errcode.as_deref() == Some("M_USER_IN_USE")
    }

    fn can_try_register(&self) -> bool {
        (400..500).contains(&self.status_code)
    }
}

pub(super) async fn create_session(
    headers: HeaderMap,
    Json(payload): Json<MatrixSessionRequest>,
) -> Result<Response> {
    let user_id = {
        let mut state = lock_state();
        let (_session, user) = match require_auth(&headers, &mut state) {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };
        user.id
    };

    let Some(homeserver) = matrix_homeserver() else {
        return Ok(chat_bootstrap_error(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            &headers,
            "Messaging service is not configured",
            "CHAT_NOT_CONFIGURED",
            None,
        ));
    };

    let device_name = normalize_device_name(payload.device_name.as_deref());
    let matrix_localpart = matrix_localpart_from_user_id(&user_id);
    let matrix_password = matrix_password_from_user_id(&user_id);

    let http_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return Ok(chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                &headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
                Some(json!({
                    "reason": "http_client_init_failed",
                    "message": error.to_string(),
                })),
            ));
        }
    };

    let auth_result = match login_matrix_user(
        &http_client,
        &homeserver,
        &matrix_localpart,
        &matrix_password,
        &device_name,
    )
    .await
    {
        Ok(session) => Ok(session),
        Err(login_error) if login_error.can_try_register() => {
            match register_matrix_user(
                &http_client,
                &homeserver,
                &matrix_localpart,
                &matrix_password,
                &device_name,
            )
            .await
            {
                Ok(session) => Ok(session),
                Err(register_error) if register_error.is_user_in_use() => {
                    login_matrix_user(
                        &http_client,
                        &homeserver,
                        &matrix_localpart,
                        &matrix_password,
                        &device_name,
                    )
                    .await
                }
                Err(register_error) => Err(register_error),
            }
        }
        Err(login_error) => Err(login_error),
    };

    let matrix_auth = match auth_result {
        Ok(session) => session,
        Err(error) => {
            tracing::warn!(
                status_code = error.status_code,
                errcode = error.errcode,
                message = error.message,
                "matrix session bootstrap failed"
            );
            return Ok(chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                &headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
                Some(json!({
                    "upstreamStatus": error.status_code,
                    "upstreamCode": error.errcode,
                })),
            ));
        }
    };

    let Some(refresh_token) = matrix_auth.refresh_token.clone() else {
        return Ok(chat_bootstrap_error(
            axum::http::StatusCode::BAD_GATEWAY,
            &headers,
            "Messaging service is temporarily unavailable",
            "CHAT_UNAVAILABLE",
            Some(json!({
                "reason": "missing_refresh_token",
            })),
        ));
    };

    let expires_at = matrix_auth
        .expires_in_ms
        .and_then(|duration| Utc::now().timestamp_millis().checked_add(duration));

    Ok(Json(MatrixSessionEnvelope {
        data: MatrixSessionData {
            homeserver,
            access_token: matrix_auth.access_token,
            refresh_token,
            user_id: matrix_auth.user_id,
            device_id: matrix_auth.device_id,
            expires_at,
        },
    })
    .into_response())
}

fn chat_bootstrap_error(
    status: axum::http::StatusCode,
    headers: &HeaderMap,
    message: &str,
    code: &'static str,
    details: Option<serde_json::Value>,
) -> Response {
    error_response(
        status,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code,
            details,
        },
    )
}

fn matrix_homeserver() -> Option<String> {
    std::env::var("MATRIX_HOMESERVER_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_device_name(name: Option<&str>) -> String {
    let trimmed = name.map_or(DEFAULT_DEVICE_NAME, str::trim);
    let bounded: String = trimmed.chars().take(64).collect();
    if bounded.is_empty() {
        DEFAULT_DEVICE_NAME.to_string()
    } else {
        bounded
    }
}

fn matrix_localpart_from_user_id(user_id: &str) -> String {
    let raw: String = user_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let normalized = raw.to_ascii_lowercase();
    if normalized.is_empty() {
        "poziomki_user".to_string()
    } else {
        format!("poziomki_{normalized}")
    }
}

fn matrix_password_from_user_id(user_id: &str) -> String {
    let pepper = std::env::var("MATRIX_PASSWORD_PEPPER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PASSWORD_PEPPER.to_string());
    format!("pz:{user_id}:{pepper}")
}

fn matrix_registration_token() -> String {
    std::env::var("MATRIX_REGISTRATION_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEV_REGISTRATION_TOKEN.to_string())
}

fn matrix_endpoint(homeserver: &str, path: &str) -> String {
    format!(
        "{}/{}",
        homeserver.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

async fn login_matrix_user(
    http_client: &reqwest::Client,
    homeserver: &str,
    user_localpart: &str,
    password: &str,
    device_name: &str,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/login");
    let payload = json!({
        "type": "m.login.password",
        "identifier": {
            "type": "m.id.user",
            "user": user_localpart,
        },
        "password": password,
        "initial_device_display_name": device_name,
        "refresh_token": true,
    });
    execute_matrix_auth_request(http_client, &url, payload).await
}

async fn register_matrix_user(
    http_client: &reqwest::Client,
    homeserver: &str,
    user_localpart: &str,
    password: &str,
    device_name: &str,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/register");
    let payload = json!({
        "username": user_localpart,
        "password": password,
        "initial_device_display_name": device_name,
        "refresh_token": true,
        "inhibit_login": false,
        "auth": {
            "type": "m.login.registration_token",
            "token": matrix_registration_token(),
        },
    });
    execute_matrix_auth_request(http_client, &url, payload).await
}

async fn execute_matrix_auth_request(
    http_client: &reqwest::Client,
    url: &str,
    payload: serde_json::Value,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let response = http_client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|error| MatrixRequestError {
            status_code: 503,
            errcode: None,
            message: error.to_string(),
        })?;

    let status = response.status();
    if status.is_success() {
        return response
            .json::<MatrixAuthResponse>()
            .await
            .map_err(|error| MatrixRequestError {
                status_code: 502,
                errcode: None,
                message: format!("invalid matrix auth response: {error}"),
            });
    }

    let status_code = status.as_u16();
    let response_text = response.text().await.unwrap_or_else(|_| String::new());
    let parsed_error = serde_json::from_str::<MatrixErrorBody>(&response_text).ok();

    Err(MatrixRequestError {
        status_code,
        errcode: parsed_error.as_ref().and_then(|body| body.errcode.clone()),
        message: parsed_error
            .and_then(|body| body.error)
            .unwrap_or(response_text),
    })
}
