use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use std::time::Duration;

const DEFAULT_DEVICE_NAME: &str = "Poziomki Mobile";

type HmacSha256 = Hmac<Sha256>;

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
pub(super) struct MatrixAuthResponse {
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
pub(super) struct MatrixRequestError {
    pub(super) status_code: u16,
    pub(super) errcode: Option<String>,
    pub(super) message: String,
}

impl MatrixRequestError {
    fn is_user_in_use(&self) -> bool {
        self.errcode.as_deref() == Some("M_USER_IN_USE")
    }

    fn can_try_register(&self) -> bool {
        (400..500).contains(&self.status_code)
    }
}

pub(super) struct MatrixConnConfig {
    localpart: String,
    password: String,
    device_name: String,
    registration_token: String,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum MatrixConfigError {
    MissingPasswordPepper,
    MissingRegistrationToken,
}

impl std::fmt::Display for MatrixConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPasswordPepper => write!(f, "missing MATRIX_PASSWORD_PEPPER"),
            Self::MissingRegistrationToken => write!(f, "missing MATRIX_REGISTRATION_TOKEN"),
        }
    }
}

pub(super) fn resolve_homeserver() -> Option<String> {
    std::env::var("MATRIX_HOMESERVER_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

/// Public-facing homeserver URL for client session responses.
/// Falls back to internal URL if public URL is not set.
pub(super) fn resolve_public_homeserver() -> Option<String> {
    super::env_non_empty("MATRIX_HOMESERVER_PUBLIC_URL")
        .or_else(|| super::env_non_empty("MATRIX_HOMESERVER_URL"))
        .map(|v| v.trim().trim_end_matches('/').to_string())
}

pub(super) fn build_conn_config(
    user_pid: &str,
    device_name: Option<&str>,
) -> std::result::Result<MatrixConnConfig, MatrixConfigError> {
    let password_pepper = super::env_non_empty("MATRIX_PASSWORD_PEPPER")
        .ok_or(MatrixConfigError::MissingPasswordPepper)?;
    let registration_token = super::env_non_empty("MATRIX_REGISTRATION_TOKEN")
        .ok_or(MatrixConfigError::MissingRegistrationToken)?;

    Ok(MatrixConnConfig {
        localpart: matrix_localpart_from_user_id(user_pid),
        password: derive_matrix_password(user_pid, &password_pepper),
        device_name: normalize_device_name(device_name),
        registration_token,
    })
}

#[allow(clippy::result_large_err)]
pub(super) fn init_http_client(
    headers: &HeaderMap,
) -> std::result::Result<reqwest::Client, Response> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_error| {
            super::matrix::chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
            )
        })
}

pub(super) async fn try_matrix_auth(
    client: &reqwest::Client,
    homeserver: &str,
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    match login_matrix_user(client, homeserver, config).await {
        Ok(session) => Ok(session),
        Err(login_error) if login_error.can_try_register() => {
            try_register_then_login(client, homeserver, config).await
        }
        Err(login_error) => Err(login_error),
    }
}

async fn try_register_then_login(
    client: &reqwest::Client,
    homeserver: &str,
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    match register_matrix_user(client, homeserver, config).await {
        Ok(session) => Ok(session),
        Err(register_error) if register_error.is_user_in_use() => {
            login_matrix_user(client, homeserver, config).await
        }
        Err(register_error) => Err(register_error),
    }
}

#[allow(clippy::result_large_err)]
pub(super) fn build_session_response(
    homeserver: String,
    auth: MatrixAuthResponse,
    headers: &HeaderMap,
) -> std::result::Result<Response, Response> {
    let Some(refresh_token) = auth.refresh_token.clone() else {
        return Err(super::matrix::chat_bootstrap_error(
            axum::http::StatusCode::BAD_GATEWAY,
            headers,
            "Messaging service is temporarily unavailable",
            "CHAT_UNAVAILABLE",
        ));
    };

    let expires_at = auth
        .expires_in_ms
        .and_then(|duration| chrono::Utc::now().timestamp_millis().checked_add(duration));

    Ok(axum::Json(MatrixSessionEnvelope {
        data: MatrixSessionData {
            homeserver,
            access_token: auth.access_token,
            refresh_token,
            user_id: auth.user_id,
            device_id: auth.device_id,
            expires_at,
        },
    })
    .into_response())
}

fn derive_matrix_password(user_pid: &str, pepper: &str) -> String {
    // HMAC accepts any key length, so new_from_slice never fails for SHA-256.
    #[allow(clippy::expect_used)]
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("HMAC-SHA256 accepts any key length");
    mac.update(user_pid.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
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
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/login");
    let payload = json!({
        "type": "m.login.password",
        "identifier": {
            "type": "m.id.user",
            "user": config.localpart.as_str(),
        },
        "password": config.password.as_str(),
        "initial_device_display_name": config.device_name.as_str(),
        "refresh_token": true,
    });
    execute_matrix_auth_request(http_client, &url, payload).await
}

async fn register_matrix_user(
    http_client: &reqwest::Client,
    homeserver: &str,
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/register");
    let payload = json!({
        "username": config.localpart.as_str(),
        "password": config.password.as_str(),
        "initial_device_display_name": config.device_name.as_str(),
        "refresh_token": true,
        "inhibit_login": false,
        "auth": {
            "type": "m.login.registration_token",
            "token": config.registration_token.as_str(),
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

    parse_matrix_error_response(response).await
}

async fn parse_matrix_error_response(
    response: reqwest::Response,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let status_code = response.status().as_u16();
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

mod hex {
    pub(super) fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut hex_string = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            use std::fmt::Write;
            let _ = write!(hex_string, "{b:02x}");
        }
        hex_string
    }
}
