use axum::{
    Router,
    http::{HeaderValue, header},
    routing::{delete, get, patch, post},
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::app::AppContext;

mod auth;
mod catalog;
mod common;
mod events;
pub(crate) mod imgproxy_signing;
mod matching;
mod matrix;
mod profiles;
mod push_gateway;
mod root;
mod search;
mod settings;
mod state;
mod uploads;

pub(crate) use common::{
    ErrorSpec, env_non_empty, error_response, extract_filename, resolve_image_url,
    resolve_image_urls, resolve_thumbhashes,
};

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
        .route("/search", get(search::search))
        .route("/messages/search", get(search::search_messages))
        .layer(cache_layer("private, max-age=60"))
}

fn matrix_config_routes() -> Router<AppContext> {
    Router::new()
        .route("/config", get(root::matrix_config))
        .layer(cache_layer("no-store"))
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
        .route("/outbox/status", get(root::outbox_status))
        .layer(cache_layer("no-store"))
}

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/health", get(root::health))
        .route("/", get(root::root))
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
    event_id: uuid::Uuid,
    profile_id: uuid::Uuid,
    leave: bool,
) -> std::result::Result<(), String> {
    if leave {
        matrix::sync_event_membership_after_leave_background(event_id, profile_id).await
    } else {
        matrix::sync_event_membership_after_attend_background(event_id, profile_id).await
    }
}

pub(crate) async fn deliver_upload_variants_generation_job(
    upload_id: uuid::Uuid,
) -> std::result::Result<(), String> {
    uploads::generate_upload_variants_job(upload_id).await
}
