use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};

type Result<T> = crate::error::AppResult<T>;

use super::super::{state::DataResponse, ErrorSpec};
use super::{
    forbidden_response, load_event_for_matrix, require_auth_profile_for_matrix, MatrixRoomData,
};

mod creation;
mod pending;

use creation::can_access_event_chat;
use creation::EventRoomRequest;
use pending::ensure_event_room;

pub(super) struct EventRoomResolution {
    pub(super) room_id: String,
    pub(super) from_existing_mapping: bool,
}

pub(super) async fn resolve_event_room(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Result<Response> {
    match do_resolve_event_room(&headers, &event_id).await {
        Ok(response) | Err(response) => Ok(response),
    }
}

async fn do_resolve_event_room(
    headers: &HeaderMap,
    event_id: &str,
) -> std::result::Result<Response, Response> {
    let (profile, user_pid) = require_auth_profile_for_matrix(headers).await?;
    let (event, event_uuid) = load_event_for_matrix(headers, event_id).await?;

    let can_access = can_access_event_chat(event_uuid, event.creator_id, profile.id)
        .await
        .map_err(|_error| {
            super::super::error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        })?;
    if !can_access {
        return Err(forbidden_response(
            headers,
            "Only event attendees can access event chat",
        ));
    }

    let room_req = EventRoomRequest {
        event_id: event_uuid,
        event_title: event.title.clone(),
        creator_profile_id: event.creator_id,
        requesting_user_pid: user_pid,
    };
    let resolution = ensure_event_room(headers, &room_req).await?;

    if !resolution.from_existing_mapping {
        super::membership::ensure_profile_joined_event_room(
            headers,
            &event,
            &profile,
            &resolution.room_id,
        )
        .await?;
    }

    Ok(Json(DataResponse {
        data: MatrixRoomData {
            room_id: resolution.room_id,
        },
    })
    .into_response())
}

pub(super) fn event_room_internal_error(headers: &HeaderMap, message: &str) -> Response {
    super::super::error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    )
}
