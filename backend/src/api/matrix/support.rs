use axum::http::HeaderMap;
use axum::response::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

mod bootstrap;
mod operations;

const DEFAULT_DEVICE_NAME: &str = "Poziomki Mobile";

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
    pub(super) access_token: String,
    pub(super) user_id: String,
    pub(super) device_id: String,
    #[serde(default)]
    pub(super) refresh_token: Option<String>,
    #[serde(default)]
    pub(super) expires_in_ms: Option<i64>,
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
    pub(super) localpart: String,
    pub(super) password: String,
    pub(super) device_name: String,
    pub(super) device_id: Option<String>,
    pub(super) registration_token: String,
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
    bootstrap::resolve_homeserver()
}

pub(super) fn resolve_public_homeserver() -> Option<String> {
    bootstrap::resolve_public_homeserver()
}

pub(super) fn build_conn_config(
    user_pid: &str,
    device_name: Option<&str>,
    device_id: Option<&str>,
) -> std::result::Result<MatrixConnConfig, MatrixConfigError> {
    bootstrap::build_conn_config(user_pid, device_name, device_id)
}

#[allow(clippy::result_large_err)]
pub(super) fn init_http_client(
    headers: &HeaderMap,
) -> std::result::Result<reqwest::Client, Response> {
    bootstrap::init_http_client(headers)
}

pub(super) async fn try_matrix_auth(
    client: &reqwest::Client,
    homeserver: &str,
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    bootstrap::try_matrix_auth(client, homeserver, config).await
}

#[allow(clippy::result_large_err)]
pub(super) fn build_session_response(
    homeserver: String,
    auth: MatrixAuthResponse,
    headers: &HeaderMap,
) -> std::result::Result<Response, Response> {
    bootstrap::build_session_response(homeserver, auth, headers)
}

pub(super) use operations::MatrixClient;

pub(super) fn content_type_from_filename(filename: &str) -> Option<&'static str> {
    operations::content_type_from_filename(filename)
}

pub(super) fn matrix_server_name_from_user_id(user_id: &str) -> Option<&str> {
    bootstrap::matrix_server_name_from_user_id(user_id)
}

pub(super) fn matrix_user_id_from_pid(user_pid: &Uuid, server_name: &str) -> String {
    bootstrap::matrix_user_id_from_pid(user_pid, server_name)
}

fn matrix_endpoint(homeserver: &str, path: &str) -> String {
    format!(
        "{}/{}",
        homeserver.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn encode_path_component(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
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

    Err(parse_matrix_error_response(response).await)
}

async fn parse_matrix_error_response(response: reqwest::Response) -> MatrixRequestError {
    let status_code = response.status().as_u16();
    let response_text = response.text().await.unwrap_or_else(|_| String::new());
    let parsed_error = serde_json::from_str::<MatrixErrorBody>(&response_text).ok();

    MatrixRequestError {
        status_code,
        errcode: parsed_error.as_ref().and_then(|body| body.errcode.clone()),
        message: parsed_error
            .and_then(|body| body.error)
            .unwrap_or(response_text),
    }
}

async fn execute_matrix_json_request<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> std::result::Result<T, MatrixRequestError> {
    let response = request.send().await.map_err(|error| MatrixRequestError {
        status_code: 503,
        errcode: None,
        message: error.to_string(),
    })?;

    let status = response.status();
    if status.is_success() {
        return response
            .json::<T>()
            .await
            .map_err(|error| MatrixRequestError {
                status_code: 502,
                errcode: None,
                message: format!("invalid matrix response: {error}"),
            });
    }

    Err(parse_matrix_error_response(response).await)
}

async fn execute_matrix_empty_request(
    request: reqwest::RequestBuilder,
) -> std::result::Result<(), MatrixRequestError> {
    let response = request.send().await.map_err(|error| MatrixRequestError {
        status_code: 503,
        errcode: None,
        message: error.to_string(),
    })?;

    if response.status().is_success() {
        return Ok(());
    }

    Err(parse_matrix_error_response(response).await)
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
