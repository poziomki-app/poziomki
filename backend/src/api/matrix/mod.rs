type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{error_response, state::require_auth_db, ErrorSpec};
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::schema::{events, profiles};

mod dm_rooms;
mod event_rooms;
mod membership;
pub(super) mod service;
mod session;

pub(super) use self::service as matrix_service;

pub(super) const PENDING_PREFIX: &str = "pending:";
pub(super) const EVENT_PENDING_RETRIES: usize = 20;
pub(super) const DM_PENDING_RETRIES: usize = 20;
pub(super) const PENDING_SLEEP_MS: u64 = 250;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixSessionRequest {
    #[serde(default)]
    device_name: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixDmRoomRequest {
    user_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatrixRoomData {
    room_id: String,
}

pub(super) struct MatrixBootstrap {
    pub(super) http_client: reqwest::Client,
    pub(super) homeserver: String,
    pub(super) auth: matrix_service::MatrixAuthResponse,
}

impl MatrixBootstrap {
    pub(super) fn client(&self) -> matrix_service::MatrixClient<'_> {
        matrix_service::MatrixClient::new(
            &self.http_client,
            &self.homeserver,
            &self.auth.access_token,
        )
    }
}

pub(super) async fn create_session(
    state: State<AppContext>,
    headers: HeaderMap,
    payload: Json<MatrixSessionRequest>,
) -> Result<Response> {
    session::create_session(state, headers, payload).await
}

pub(super) async fn resolve_event_room(
    state: State<AppContext>,
    headers: HeaderMap,
    event_id: Path<String>,
) -> Result<Response> {
    event_rooms::resolve_event_room(state, headers, event_id).await
}

pub(super) async fn resolve_dm_room(
    state: State<AppContext>,
    headers: HeaderMap,
    payload: Json<MatrixDmRoomRequest>,
) -> Result<Response> {
    dm_rooms::resolve_dm_room(state, headers, payload).await
}

pub(super) async fn sync_event_membership_after_attend_background(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|e| e.to_string())?;

    let event = events::table
        .find(event_id)
        .first::<Event>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("event not found for membership sync: {event_id}"))?;
    let profile = profiles::table
        .find(profile_id)
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("profile not found for membership sync: {profile_id}"))?;

    let headers = HeaderMap::new();
    membership::sync_event_membership_after_attend_result(&headers, &event, &profile).await
}

pub(super) async fn sync_event_membership_after_leave_background(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|e| e.to_string())?;

    let event = events::table
        .find(event_id)
        .first::<Event>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("event not found for membership leave sync: {event_id}"))?;
    let profile = profiles::table
        .find(profile_id)
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("profile not found for membership leave sync: {profile_id}"))?;

    let headers = HeaderMap::new();
    membership::sync_event_membership_after_leave_result(&headers, &event, &profile).await
}

pub(super) async fn sync_profile_avatar_best_effort(
    user_pid: &Uuid,
    profile_picture_filename: Option<&str>,
) {
    session::sync_profile_avatar_best_effort(user_pid, profile_picture_filename).await;
}

pub(super) async fn bootstrap_matrix_auth(
    user_pid: &str,
    headers: &HeaderMap,
    device_name: Option<&str>,
    device_id: Option<&str>,
) -> std::result::Result<MatrixBootstrap, Response> {
    let homeserver = matrix_service::resolve_homeserver().ok_or_else(|| {
        chat_bootstrap_error(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            "Messaging service is not configured",
            "CHAT_NOT_CONFIGURED",
        )
    })?;

    let config =
        matrix_service::build_conn_config(user_pid, device_name, device_id).map_err(|error| {
            tracing::warn!(%error, "matrix bootstrap is not configured");
            chat_bootstrap_error(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                headers,
                "Messaging service is not configured",
                "CHAT_NOT_CONFIGURED",
            )
        })?;
    let http_client = matrix_service::init_http_client(headers)?;
    let auth = matrix_service::try_matrix_auth(&http_client, &homeserver, &config)
        .await
        .map_err(|error| {
            tracing::warn!(
                status_code = error.status_code,
                errcode = error.errcode,
                message = error.message,
                "matrix bootstrap failed"
            );
            chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
            )
        })?;

    Ok(MatrixBootstrap {
        http_client,
        homeserver,
        auth,
    })
}

pub(super) fn is_matrix_room_id(value: &str) -> bool {
    value.starts_with('!')
}

pub(super) fn build_pending_token() -> String {
    format!("{PENDING_PREFIX}{}", Uuid::new_v4().simple())
}

pub(super) async fn require_auth_profile_for_matrix(
    headers: &HeaderMap,
) -> std::result::Result<(Profile, Uuid), Response> {
    let (_session, user) = require_auth_db(headers)
        .await
        .map_err(|response| *response)?;

    let mut conn = crate::db::conn()
        .await
        .map_err(|_e| profile_not_found_response(headers))?;

    let profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|_error| profile_not_found_response(headers))?
        .ok_or_else(|| profile_not_found_response(headers))?;
    Ok((profile, user.pid))
}

pub(super) async fn load_event_for_matrix(
    headers: &HeaderMap,
    event_id: &str,
) -> std::result::Result<(Event, Uuid), Response> {
    let event_uuid = Uuid::parse_str(event_id).map_err(|_error| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Invalid event ID".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        )
    })?;

    let mut conn = crate::db::conn().await.map_err(|_e| {
        error_response(
            axum::http::StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: format!("Event '{event_id}' not found"),
                code: "NOT_FOUND",
                details: None,
            },
        )
    })?;

    let event = events::table
        .find(event_uuid)
        .first::<Event>(&mut conn)
        .await
        .optional()
        .map_err(|_error| {
            error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: format!("Event '{event_id}' not found"),
                    code: "NOT_FOUND",
                    details: None,
                },
            )
        })?
        .ok_or_else(|| {
            error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: format!("Event '{event_id}' not found"),
                    code: "NOT_FOUND",
                    details: None,
                },
            )
        })?;

    Ok((event, event_uuid))
}

fn profile_not_found_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: "Profile not found. Create a profile first.".to_string(),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

pub(super) fn forbidden_response(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::FORBIDDEN,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "FORBIDDEN",
            details: None,
        },
    )
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
