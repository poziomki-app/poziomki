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
use pending::ensure_event_room;

pub(super) struct EventRoomResolution {
    pub(super) room_id: String,
    pub(super) from_existing_mapping: bool,
}

pub(super) async fn resolve_event_room(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Result<Response> {
    let (profile, user_pid) = match require_auth_profile_for_matrix(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(response),
    };
    let (event, event_uuid) = match load_event_for_matrix(&ctx.db, &headers, &event_id).await {
        Ok(found) => found,
        Err(response) => return Ok(response),
    };

    let can_access =
        can_access_event_chat(&ctx.db, event_uuid, event.creator_id, profile.id).await?;
    if !can_access {
        return Ok(forbidden_response(
            &headers,
            "Only event attendees can access event chat",
        ));
    }

    let resolution = match ensure_event_room(
        &ctx.db,
        &headers,
        event_uuid,
        &event.title,
        event.creator_id,
        user_pid,
    )
    .await
    {
        Ok(room) => room,
        Err(response) => return Ok(response),
    };

    if !resolution.from_existing_mapping {
        match super::membership::ensure_profile_joined_event_room(
            &ctx.db,
            &headers,
            &event,
            &profile,
            &resolution.room_id,
        )
        .await
        {
            Ok(()) => {}
            Err(response) => return Ok(response),
        }
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
