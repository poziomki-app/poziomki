use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};
use serde::Serialize;
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

use crate::app::AppContext;

type Result<T> = crate::error::AppResult<T>;

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

fn auth_routes() -> Router<AppContext> {
    Router::new()
        .route("/get-session", get(auth::get_session))
        .route("/sign-up/email", post(auth::sign_up))
        .route("/sign-in/email", post(auth::sign_in))
        .route("/verify-otp", post(auth::verify_otp))
        .route("/resend-otp", post(auth::resend_otp))
        .route("/sign-out", post(auth::sign_out))
        .route("/sessions", get(auth::sessions))
        .route("/account", delete(auth::delete_account))
        .route("/export", get(auth::export_data))
}

fn profiles_routes() -> Router<AppContext> {
    Router::new()
        .route("/me", get(profiles::profile_me))
        .route("/", post(profiles::profile_create))
        .route("/{id}", get(profiles::profile_get))
        .route("/{id}", patch(profiles::profile_update))
        .route("/{id}", delete(profiles::profile_delete))
        .route("/{id}/full", get(profiles::profile_get_full))
        .layer(cache_layer("private, max-age=60"))
}

fn degrees_routes() -> Router<AppContext> {
    Router::new()
        .route("/", get(catalog::degrees_search))
        .layer(cache_layer("public, max-age=1800"))
}

fn tags_routes() -> Router<AppContext> {
    Router::new()
        .route("/", get(catalog::tags_search).post(catalog::tags_create))
        .layer(cache_layer("public, max-age=1800"))
}

fn events_routes() -> Router<AppContext> {
    Router::new()
        .route("/", get(events::events_list).post(events::event_create))
        .route("/mine", get(events::events_mine))
        .route("/{id}", get(events::event_get))
        .route("/{id}", patch(events::event_update))
        .route("/{id}", delete(events::event_delete))
        .route("/{id}/attendees", get(events::event_attendees))
        .route("/{id}/attend", post(events::event_attend))
        .route("/{id}/attend", delete(events::event_leave))
        .layer(cache_layer("private, max-age=60"))
}

fn matching_routes() -> Router<AppContext> {
    Router::new()
        .route("/profiles", get(matching::profiles_recommendations))
        .route("/events", get(matching::events_recommendations))
        .layer(cache_layer("private, max-age=300"))
}

fn uploads_routes() -> Router<AppContext> {
    Router::new()
        .route("/auth-check", get(uploads::auth_check))
        .route("/presign", post(uploads::file_upload_presign))
        .route("/complete", post(uploads::file_upload_complete))
        .route("/{filename}/status", get(uploads::file_status))
        .route("/", post(uploads::file_upload))
        .route("/{filename}", get(uploads::file_get))
        .route("/{filename}", delete(uploads::file_delete))
}

fn settings_routes() -> Router<AppContext> {
    Router::new().route(
        "/",
        get(settings::settings_get).patch(settings::settings_update),
    )
}

fn search_routes() -> Router<AppContext> {
    Router::new()
        .route("/search", get(search_api::search))
        .layer(cache_layer("private, max-age=60"))
}

fn matrix_config_routes() -> Router<AppContext> {
    Router::new()
        .route("/config", get(matrix_config))
        .layer(cache_layer("public, max-age=3600"))
}

fn matrix_session_routes() -> Router<AppContext> {
    Router::new()
        .route("/session", post(matrix::create_session))
        .layer(cache_layer("no-store"))
}

fn matrix_room_routes() -> Router<AppContext> {
    Router::new()
        .route("/events/{eventId}/room", get(matrix::resolve_event_room))
        .route("/dms", post(matrix::resolve_dm_room))
        .layer(cache_layer("no-store"))
}

fn push_gateway_routes() -> Router<AppContext> {
    Router::new().route("/notify", post(push_gateway::notify))
}

fn ops_routes() -> Router<AppContext> {
    Router::new()
        .route("/outbox/status", get(outbox_status))
        .layer(cache_layer("no-store"))
}

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/health", get(health))
        .route("/", get(root))
        .nest("/api/v1/auth", auth_routes())
        .nest("/api/v1/profiles", profiles_routes())
        .nest("/api/v1/degrees", degrees_routes())
        .nest("/api/v1/tags", tags_routes())
        .nest("/api/v1/events", events_routes())
        .nest("/api/v1/matching", matching_routes())
        .nest("/api/v1/uploads", uploads_routes())
        .nest("/api/v1/settings", settings_routes())
        .nest("/api/v1", search_routes())
        .nest("/api/v1/matrix", matrix_config_routes())
        .nest("/api/v1/matrix", matrix_session_routes())
        .nest("/api/v1/matrix", matrix_room_routes())
        .nest("/_matrix/push/v1", push_gateway_routes())
        .nest("/api/v1/ops", ops_routes())
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
