use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use loco_rs::prelude::*;
use serde::Serialize;
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

mod auth;
mod catalog;
mod events;
mod matching;
mod matrix;
mod matrix_support;
mod profiles;
mod push_gateway;
mod search_api;
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
#[serde(rename_all = "camelCase")]
struct OutboxStatusResponse {
    status: &'static str,
    metrics: crate::tasks::OutboxStatsSnapshot,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigResponse {
    data: MatrixConfigData,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixConfigData {
    homeserver: Option<String>,
    chat_mode: &'static str,
    push_gateway_url: Option<String>,
    ntfy_server: Option<String>,
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

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
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

/// Resolve multiple image URLs in parallel.
async fn resolve_image_urls(stored: &[String]) -> Vec<String> {
    let futs: Vec<_> = stored.iter().map(|s| resolve_image_url(s)).collect();
    futures::future::join_all(futs).await
}

async fn health() -> Result<Response> {
    Ok(Json(HealthResponse { status: "ok" }).into_response())
}

fn ops_status_token() -> Option<String> {
    env_non_empty("OPS_STATUS_TOKEN")
}

fn ops_token_matches(headers: &HeaderMap) -> bool {
    let Some(expected) = ops_status_token() else {
        return false;
    };
    headers
        .get("x-ops-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| actual == expected)
}

async fn outbox_status(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    if ops_status_token().is_none() {
        return Ok((axum::http::StatusCode::NOT_FOUND, "not found").into_response());
    }

    if !ops_token_matches(&headers) {
        return Ok((axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response());
    }

    let metrics = crate::tasks::outbox_stats_snapshot(&ctx.db).await?;
    let status = if metrics.failed_jobs > 0 || metrics.oldest_ready_job_age_seconds > 60 {
        "degraded"
    } else {
        "ok"
    };

    Ok(Json(OutboxStatusResponse { status, metrics }).into_response())
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
    let homeserver = env_non_empty("MATRIX_HOMESERVER_PUBLIC_URL")
        .or_else(|| env_non_empty("MATRIX_HOMESERVER_URL"));
    let push_gateway_url = env_non_empty("PUSH_GATEWAY_URL");
    let ntfy_server = env_non_empty("NTFY_SERVER_URL");
    Ok(Json(MatrixConfigResponse {
        data: MatrixConfigData {
            homeserver,
            chat_mode: "matrix-native",
            push_gateway_url,
            ntfy_server,
        },
    })
    .into_response())
}

fn cache_layer(value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(header::CACHE_CONTROL, HeaderValue::from_static(value))
}

fn auth_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/auth")
        .add("/get-session", get(auth::get_session))
        .add("/sign-up/email", post(auth::sign_up))
        .add("/sign-in/email", post(auth::sign_in))
        .add("/verify-otp", post(auth::verify_otp))
        .add("/resend-otp", post(auth::resend_otp))
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
        .layer(cache_layer("private, max-age=60"))
}

fn degrees_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/degrees")
        .add("", get(catalog::degrees_search))
        .layer(cache_layer("public, max-age=1800"))
}

fn tags_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/tags")
        .add("", get(catalog::tags_search))
        .add("", post(catalog::tags_create))
        .layer(cache_layer("public, max-age=1800"))
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
        .layer(cache_layer("private, max-age=60"))
}

fn matching_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matching")
        .add("/profiles", get(matching::profiles_recommendations))
        .add("/events", get(matching::events_recommendations))
        .layer(cache_layer("private, max-age=300"))
}

fn uploads_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/uploads")
        .add("/auth-check", get(uploads::auth_check))
        .add("/presign", post(uploads::file_upload_presign))
        .add("/complete", post(uploads::file_upload_complete))
        .add("/{filename}/status", get(uploads::file_status))
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

fn search_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1")
        .add("/search", get(search_api::search))
        .layer(cache_layer("private, max-age=60"))
}

fn matrix_config_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matrix")
        .add("/config", get(matrix_config))
        .layer(cache_layer("public, max-age=3600"))
}

fn matrix_session_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matrix")
        .add("/session", post(matrix::create_session))
        .layer(cache_layer("no-store"))
}

fn matrix_room_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/matrix")
        .add("/events/{eventId}/room", get(matrix::resolve_event_room))
        .add("/dms", post(matrix::resolve_dm_room))
        .layer(cache_layer("no-store"))
}

fn push_gateway_routes() -> Routes {
    Routes::new()
        .prefix("/_matrix/push/v1")
        .add("/notify", post(push_gateway::notify))
}

fn ops_routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/ops")
        .add("/outbox/status", get(outbox_status))
        .layer(cache_layer("no-store"))
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
        search_routes(),
        matrix_config_routes(),
        matrix_session_routes(),
        matrix_room_routes(),
        push_gateway_routes(),
        ops_routes(),
    ]
}

pub(crate) async fn deliver_otp_email_job(to: &str, code: &str) {
    auth::deliver_otp_email_job(to, code).await;
}

pub(crate) async fn deliver_matrix_profile_avatar_sync_job(
    user_pid: &uuid::Uuid,
    profile_picture_filename: Option<&str>,
) {
    matrix::sync_profile_avatar_best_effort(user_pid, profile_picture_filename).await;
}

pub(crate) async fn deliver_matrix_event_membership_sync_job(
    db: &sea_orm::DatabaseConnection,
    event_id: uuid::Uuid,
    profile_id: uuid::Uuid,
    leave: bool,
) -> std::result::Result<(), String> {
    if leave {
        matrix::sync_event_membership_after_leave_background(db, event_id, profile_id).await
    } else {
        matrix::sync_event_membership_after_attend_background(db, event_id, profile_id).await
    }
}

pub(crate) async fn deliver_upload_variants_generation_job(
    db: &sea_orm::DatabaseConnection,
    upload_id: uuid::Uuid,
) -> std::result::Result<(), String> {
    uploads::generate_upload_variants_job(db, upload_id).await
}
