use std::time::Duration;

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::Response;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

use super::{
    execute_matrix_auth_request, matrix_endpoint, MatrixAuthResponse, MatrixConfigError,
    MatrixConnConfig, MatrixErrorBody, MatrixRequestError, MatrixSessionData,
    MatrixSessionEnvelope, DEFAULT_DEVICE_NAME,
};

type HmacSha256 = Hmac<Sha256>;

/// UIA (User-Interactive Authentication) response from Synapse.
/// Returned with HTTP 401 when additional auth stages are needed.
#[derive(Clone, Debug, Deserialize)]
struct UiaResponse {
    session: String,
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
    crate::api::env_non_empty("MATRIX_HOMESERVER_PUBLIC_URL")
        .or_else(|| crate::api::env_non_empty("MATRIX_HOMESERVER_URL"))
        .map(|v| v.trim().trim_end_matches('/').to_string())
}

pub(super) fn build_conn_config(
    user_pid: &str,
    device_name: Option<&str>,
    device_id: Option<&str>,
) -> std::result::Result<MatrixConnConfig, MatrixConfigError> {
    let password_pepper = crate::api::env_non_empty("MATRIX_PASSWORD_PEPPER")
        .ok_or(MatrixConfigError::MissingPasswordPepper)?;
    let registration_token = crate::api::env_non_empty("MATRIX_REGISTRATION_TOKEN")
        .ok_or(MatrixConfigError::MissingRegistrationToken)?;

    Ok(MatrixConnConfig {
        localpart: matrix_localpart_from_user_id(user_pid),
        password: derive_matrix_password(user_pid, &password_pepper),
        device_name: normalize_device_name(device_name),
        device_id: normalize_device_id(device_id),
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
            super::super::chat_bootstrap_error(
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
        return Err(super::super::chat_bootstrap_error(
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
    super::hex::encode(result.into_bytes())
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

/// Pass the device ID through with minimal sanitisation.
/// The mobile client generates clean IDs (e.g. `POZ<hex>`); uppercasing or
/// stripping characters here caused mismatches with the SDK crypto store
/// which is keyed by the exact device ID returned by the homeserver.
fn normalize_device_id(device_id: Option<&str>) -> Option<String> {
    let trimmed = device_id.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return None;
    }

    // Matrix spec allows printable ASCII in device IDs; just length-bound.
    let bounded: String = trimmed.chars().take(64).collect();
    if bounded.is_empty() {
        return None;
    }

    Some(bounded)
}

pub(super) fn matrix_localpart_from_user_id(user_id: &str) -> String {
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

pub(super) fn matrix_server_name_from_user_id(user_id: &str) -> Option<&str> {
    user_id
        .strip_prefix('@')
        .and_then(|without_at| without_at.split_once(':').map(|(_, server)| server))
        .filter(|server| !server.is_empty())
}

pub(super) fn matrix_user_id_from_pid(user_pid: &Uuid, server_name: &str) -> String {
    let localpart = matrix_localpart_from_user_id(&user_pid.to_string());
    format!("@{localpart}:{server_name}")
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
    let payload = with_device_id(payload, config.device_id.as_deref());
    execute_matrix_auth_request(http_client, &url, payload).await
}

async fn register_matrix_user(
    http_client: &reqwest::Client,
    homeserver: &str,
    config: &MatrixConnConfig,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/register");

    // Step 1: Send registration with token auth.
    // Some homeservers (Tuwunel) complete in one step.
    // Synapse uses UIA and may require additional stages (e.g. m.login.dummy).
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
    let payload = with_device_id(payload, config.device_id.as_deref());

    match execute_matrix_register_request(http_client, &url, &payload).await {
        Ok(result) => Ok(result),
        Err(RegisterStepResult::UiaNeeded(uia)) => {
            // Step 2: Complete remaining UIA stages (typically m.login.dummy).
            complete_uia_registration(http_client, &url, &payload, &uia).await
        }
        Err(RegisterStepResult::Error(e)) => Err(e),
    }
}

/// Completes UIA registration by sending dummy auth for any remaining stages.
async fn complete_uia_registration(
    http_client: &reqwest::Client,
    url: &str,
    base_payload: &serde_json::Value,
    uia: &UiaResponse,
) -> std::result::Result<MatrixAuthResponse, MatrixRequestError> {
    let mut payload = base_payload.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "auth".to_string(),
            json!({
                "type": "m.login.dummy",
                "session": uia.session,
            }),
        );
    }
    match execute_matrix_register_request(http_client, url, &payload).await {
        Ok(result) => Ok(result),
        Err(RegisterStepResult::UiaNeeded(_)) => Err(MatrixRequestError {
            status_code: 502,
            errcode: None,
            message: "UIA registration did not complete after dummy stage".to_string(),
        }),
        Err(RegisterStepResult::Error(e)) => Err(e),
    }
}

enum RegisterStepResult {
    UiaNeeded(UiaResponse),
    Error(MatrixRequestError),
}

/// Sends a registration request and distinguishes between success, UIA challenge, and error.
async fn execute_matrix_register_request(
    http_client: &reqwest::Client,
    url: &str,
    payload: &serde_json::Value,
) -> std::result::Result<MatrixAuthResponse, RegisterStepResult> {
    let response = http_client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|error| {
            RegisterStepResult::Error(MatrixRequestError {
                status_code: 503,
                errcode: None,
                message: error.to_string(),
            })
        })?;

    let status = response.status();
    if status.is_success() {
        return response
            .json::<MatrixAuthResponse>()
            .await
            .map_err(|error| {
                RegisterStepResult::Error(MatrixRequestError {
                    status_code: 502,
                    errcode: None,
                    message: format!("invalid matrix auth response: {error}"),
                })
            });
    }

    // HTTP 401 with a session field = UIA challenge, not a real error.
    let status_code = status.as_u16();
    let body_text = response.text().await.unwrap_or_else(|_| String::new());

    let uia_challenge = (status_code == 401)
        .then(|| serde_json::from_str::<UiaResponse>(&body_text).ok())
        .flatten()
        .filter(|uia| !uia.session.is_empty());
    if let Some(uia) = uia_challenge {
        return Err(RegisterStepResult::UiaNeeded(uia));
    }

    let parsed_error = serde_json::from_str::<MatrixErrorBody>(&body_text).ok();
    Err(RegisterStepResult::Error(MatrixRequestError {
        status_code,
        errcode: parsed_error.as_ref().and_then(|body| body.errcode.clone()),
        message: parsed_error
            .and_then(|body| body.error)
            .unwrap_or(body_text),
    }))
}

fn with_device_id(mut payload: serde_json::Value, device_id: Option<&str>) -> serde_json::Value {
    if let Some(device_id) = device_id {
        if let Some(object) = payload.as_object_mut() {
            object.insert("device_id".to_string(), json!(device_id));
        }
    }
    payload
}
