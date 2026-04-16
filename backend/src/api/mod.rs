use axum::{
    extract::MatchedPath,
    http::{header, HeaderValue},
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::app::AppContext;

mod auth;
mod catalog;
pub(crate) mod chat;
mod common;
mod events;
pub(crate) mod imgproxy_signing;
mod matching;
mod profiles;
mod root;
mod search;
mod settings;
mod state;
mod uploads;
mod xp;

pub(crate) use common::{
    auth_or_respond, env_non_empty, error_response, extract_filename, parse_uuid,
    parse_uuid_response, redact_email, resolve_bio_image_urls, resolve_image_url,
    resolve_image_urls, resolve_thumbhashes, ErrorSpec,
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
        .route("/account/password", patch(auth::change_password))
        .route("/export", get(auth::export_data))
        .route("/forgot-password", post(auth::forgot_password))
        .route(
            "/forgot-password/verify",
            post(auth::forgot_password_verify),
        )
        .route(
            "/forgot-password/resend",
            post(auth::forgot_password_resend),
        )
        .route("/reset-password", post(auth::reset_password))
}

fn profiles_routes() -> Router<AppContext> {
    // Every route here returns user-private data (own profile, bookmark
    // state, blocks, or another user's profile annotated with "is blocked by
    // me" flags). Default to no-store to keep that data off shared caches.
    Router::new()
        .route("/me", get(profiles::profile_me))
        .route("/bookmarked", get(profiles::profiles_bookmarked_handler))
        .route("/", post(profiles::profile_create))
        .route("/{id}", get(profiles::profile_get))
        .route("/{id}", patch(profiles::profile_update))
        .route("/{id}", delete(profiles::profile_delete))
        .route("/{id}/full", get(profiles::profile_get_full))
        .route(
            "/{id}/bookmark",
            post(profiles::profile_bookmark_handler).delete(profiles::profile_unbookmark_handler),
        )
        .route(
            "/{id}/block",
            post(profiles::profile_block_handler).delete(profiles::profile_unblock_handler),
        )
        .layer(cache_layer("no-store"))
}

fn degrees_routes() -> Router<AppContext> {
    Router::new()
        .route("/", get(catalog::degrees_search))
        .layer(cache_layer("public, max-age=1800"))
}

fn tags_routes() -> Router<AppContext> {
    let cached = Router::new()
        .route("/", get(catalog::tags_search).post(catalog::tags_create))
        .layer(cache_layer("public, max-age=1800"));

    Router::new()
        .route("/suggestions", post(catalog::tags_suggestions))
        .merge(cached)
}

fn events_routes() -> Router<AppContext> {
    // Personal feeds (/mine, /saved) must never sit in a shared cache.
    let personal = Router::new()
        .route("/mine", get(events::events_mine))
        .route("/saved", get(events::events_saved))
        .layer(cache_layer("no-store"));

    // Public-ish event metadata — returned to any authenticated user — may
    // still enjoy a short private cache window to smooth over repeated reads
    // from the mobile app.
    let shared = Router::new()
        .route("/", get(events::events_list).post(events::event_create))
        .route("/{id}", get(events::event_get))
        .route("/{id}", patch(events::event_update))
        .route("/{id}", delete(events::event_delete))
        .route("/{id}/attendees", get(events::event_attendees))
        .route("/{id}/attend", post(events::event_attend))
        .route("/{id}/attend", delete(events::event_leave))
        .route("/{id}/save", post(events::event_save))
        .route("/{id}/save", delete(events::event_unsave))
        .route(
            "/{id}/attendees/{profile_id}/approve",
            post(events::event_approve_attendee),
        )
        .route(
            "/{id}/attendees/{profile_id}/reject",
            post(events::event_reject_attendee),
        )
        .route("/{id}/report", post(events::event_report))
        .layer(cache_layer("private, max-age=60"));

    personal.merge(shared)
}

fn matching_routes() -> Router<AppContext> {
    // Recommendations are derived from the caller's profile/history — never
    // cache them, even privately (any proxy misbehaviour could cross-serve
    // recommendations between users).
    Router::new()
        .route("/profiles", get(matching::profiles_recommendations))
        .route("/events", get(matching::events_recommendations))
        .route("/events/{id}/feedback", post(matching::event_feedback))
        .layer(cache_layer("no-store"))
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
    // Search results reflect the caller's permissions (messages they can
    // see, events they can discover) — treat as per-user secret material.
    Router::new()
        .route("/search", get(search::search))
        .route("/messages/search", get(search::search_messages))
        .layer(cache_layer("no-store"))
}

fn chat_routes() -> Router<AppContext> {
    Router::new()
        .route(
            "/ws",
            get(chat::ws_upgrade).route_layer(middleware::from_fn(chat::ws_upgrade_gate)),
        )
        .route("/config", get(chat::chat_config))
        .route("/dms", post(chat::resolve_dm))
        .route(
            "/events/{eventId}/conversation",
            get(chat::resolve_event_conversation),
        )
        .route(
            "/conversations/{id}/report",
            post(chat::report_handler::conversation_report),
        )
        .route("/push/register", post(chat::push_register))
        .route("/push/unregister", post(chat::push_unregister))
        .layer(cache_layer("no-store"))
}

#[derive(Clone)]
struct StatusAwareOnResponse;

impl<B> tower_http::trace::OnResponse<B> for StatusAwareOnResponse {
    fn on_response(
        self,
        response: &axum::http::Response<B>,
        latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        if span.is_disabled() {
            return;
        }
        let status = response.status().as_u16();
        let latency_ms = latency.as_millis();
        if status >= 500 {
            tracing::error!(parent: span, status, latency_ms, "response");
        } else if status >= 400 {
            tracing::warn!(parent: span, status, latency_ms, "response");
        } else {
            tracing::info!(parent: span, status, latency_ms, "response");
        }
    }
}

fn ops_routes() -> Router<AppContext> {
    Router::new()
        .route("/outbox/status", get(root::outbox_status))
        .layer(cache_layer("no-store"))
}

async fn observe_http_metrics(
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let method = request.method().to_string();
    let route = crate::telemetry::metrics_route_label(
        request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str),
    )
    .to_string();
    let started_at = std::time::Instant::now();
    let response = next.run(request).await;
    crate::telemetry::record_http_request(
        &method,
        &route,
        response.status().as_u16(),
        started_at.elapsed(),
    );
    response
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
        .nest("/api/v1/chat", chat_routes())
        .nest("/api/v1/xp", xp::handler::routes())
        .nest("/api/v1/ops", ops_routes())
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(middleware::from_fn(observe_http_metrics))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &axum::http::Request<_>| {
                    if req.uri().path() == "/health" {
                        return tracing::Span::none();
                    }
                    let request_id = req
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .map_or_else(
                            || {
                                let (hi, _) = uuid::Uuid::new_v4().as_u64_pair();
                                format!("{:08x}", (hi >> 32) as u32)
                            },
                            String::from,
                        );
                    tracing::info_span!(
                        "request",
                        method = %req.method(),
                        path = %req.uri().path(),
                        request_id = %request_id,
                        user_id = tracing::field::Empty,
                    )
                })
                .on_response(StatusAwareOnResponse),
        )
}

pub(crate) async fn deliver_otp_email_job(to: &str, code: &str) -> std::result::Result<(), String> {
    auth::deliver_otp_email_job(to, code).await
}

pub(crate) async fn deliver_chat_membership_sync_job(
    event_id: uuid::Uuid,
    profile_id: uuid::Uuid,
    leave: bool,
) -> std::result::Result<(), String> {
    chat::conversations::sync_event_membership(event_id, profile_id, leave).await
}

pub(crate) async fn deliver_upload_variants_generation_job(
    upload_id: uuid::Uuid,
) -> std::result::Result<(), String> {
    uploads::generate_upload_variants_job(upload_id).await
}
