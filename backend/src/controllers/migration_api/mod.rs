use axum::{http::HeaderMap, response::IntoResponse, Json};
use loco_rs::prelude::*;
use serde::Serialize;
use uuid::Uuid;

mod auth;
mod catalog;
mod events;
mod matching;
mod matrix;
mod matrix_support;
mod profiles;
mod settings;
mod state;
mod uploads;

#[derive(Clone, Debug, Serialize)]
struct RootInfoResponse {
    docs: &'static str,
    message: &'static str,
    version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigResponse {
    data: MatrixConfigData,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigData {
    homeserver: Option<String>,
    chat_mode: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: &'static str,
    #[serde(rename = "requestId")]
    request_id: String,
    details: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub(crate) struct ErrorSpec {
    pub(crate) error: String,
    pub(crate) code: &'static str,
    pub(crate) details: Option<serde_json::Value>,
}

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), ToOwned::to_owned)
}

pub(crate) fn error_response(
    status: axum::http::StatusCode,
    headers: &HeaderMap,
    spec: ErrorSpec,
) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: spec.error,
            code: spec.code,
            request_id: request_id(headers),
            details: spec.details,
        }),
    )
        .into_response()
}

/// Strip a presigned URL down to just the filename (last path segment).
/// If the value is already a plain filename, return it unchanged.
fn extract_filename(value: &str) -> String {
    if value.starts_with("http") {
        url::Url::parse(value)
            .ok()
            .and_then(|u| u.path_segments()?.next_back().map(ToString::to_string))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| value.to_string())
    } else {
        value.to_string()
    }
}

/// Resolve a stored image value (filename or legacy presigned URL) to a fresh signed URL.
async fn resolve_image_url(stored: &str) -> String {
    let filename = extract_filename(stored);
    uploads::uploads_storage::signed_get_url(&filename)
        .await
        .unwrap_or(filename)
}

async fn health() -> Result<Response> {
    Ok(Json(HealthResponse { status: "ok" }).into_response())
}

async fn root() -> Result<Response> {
    Ok(Json(RootInfoResponse {
        docs: "/api/docs",
        message: "poziomki API v1",
        version: "1.0.0",
    })
    .into_response())
}

async fn matrix_config() -> Result<Response> {
    let homeserver = std::env::var("MATRIX_HOMESERVER_URL").ok();
    Ok(Json(MatrixConfigResponse {
        data: MatrixConfigData {
            homeserver,
            chat_mode: "matrix-native",
        },
    })
    .into_response())
}

async fn not_implemented(headers: HeaderMap) -> Result<Response> {
    Ok(error_response(
        axum::http::StatusCode::NOT_IMPLEMENTED,
        &headers,
        ErrorSpec {
            error: "Endpoint is not implemented in Rust yet".to_string(),
            code: "NOT_IMPLEMENTED",
            details: None,
        },
    ))
}

fn auth_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/auth")
        .add("/get-session", get(auth::get_session))
        .add("/sign-up/email", post(auth::sign_up))
        .add("/sign-in/email", post(auth::sign_in))
        .add("/verify-otp", post(auth::verify_otp))
        .add("/resend-otp", post(auth::resend_otp))
        .add("/email-otp/verify-email", post(auth::verify_otp))
        .add("/email-otp/send-verification-otp", post(auth::resend_otp))
        .add("/sign-out", post(auth::sign_out))
        .add("/sessions", get(auth::sessions))
        .add("/account", delete(auth::delete_account))
        .add("/export", get(auth::export_data))
}

fn profiles_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/profiles")
        .add("/me", get(profiles::profile_me))
        .add("", post(profiles::profile_create))
        .add("/{id}", get(profiles::profile_get))
        .add("/{id}", patch(profiles::profile_update))
        .add("/{id}", delete(profiles::profile_delete))
        .add("/{id}/full", get(profiles::profile_get_full))
}

fn degrees_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/degrees")
        .add("", get(catalog::degrees_search))
}

fn tags_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/tags")
        .add("", get(catalog::tags_search))
        .add("", post(catalog::tags_create))
}

fn events_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/events")
        .add("", get(events::events_list))
        .add("", post(events::event_create))
        .add("/mine", get(events::events_mine))
        .add("/{id}", get(events::event_get))
        .add("/{id}", patch(events::event_update))
        .add("/{id}", delete(events::event_delete))
        .add("/{id}/attendees", get(events::event_attendees))
        .add("/{id}/attend", post(events::event_attend))
        .add("/{id}/attend", delete(events::event_leave))
}

fn matching_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matching")
        .add("/profiles", get(matching::profiles_recommendations))
}

fn uploads_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/uploads")
        .add("/auth-check", get(uploads::auth_check))
        .add("", post(uploads::file_upload))
        .add("/{filename}", get(uploads::file_get))
        .add("/{filename}", delete(uploads::file_delete))
}

fn settings_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/settings")
        .add("", get(settings::settings_get))
        .add("", patch(settings::settings_update))
}

fn matrix_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matrix")
        .add("/config", get(matrix_config))
        .add("/session", post(matrix::create_session))
        .add("/events/{eventId}/room", get(not_implemented))
}

pub(crate) fn reset_state() {
    state::reset_state();
}

pub fn routes() -> Vec<Routes> {
    vec![
        Routes::new()
            .add("/health", get(health))
            .add("/", get(root)),
        auth_routes(),
        profiles_routes(),
        degrees_routes(),
        tags_routes(),
        events_routes(),
        matching_routes(),
        uploads_routes(),
        settings_routes(),
        matrix_routes(),
    ]
}
