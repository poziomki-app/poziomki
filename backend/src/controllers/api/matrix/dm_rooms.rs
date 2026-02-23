type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::super::{
    error_response,
    state::{require_auth_db, DataResponse},
    ErrorSpec,
};
use super::{MatrixDmRoomRequest, MatrixRoomData};
use crate::db::models::users::User;
use crate::db::schema::users;

mod creation;
mod pending;

use pending::ensure_dm_room;

pub(super) async fn resolve_dm_room(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<MatrixDmRoomRequest>,
) -> Result<Response> {
    let (own_pid, target_pid) = match validate_dm_request(&headers, &payload).await {
        Ok(pair) => pair,
        Err(response) => return Ok(response),
    };

    let room_id = match ensure_dm_room(&headers, own_pid, target_pid).await {
        Ok(room_id) => room_id,
        Err(response) => return Ok(response),
    };

    Ok(Json(DataResponse {
        data: MatrixRoomData { room_id },
    })
    .into_response())
}

async fn validate_dm_request(
    headers: &HeaderMap,
    payload: &MatrixDmRoomRequest,
) -> std::result::Result<(Uuid, Uuid), Response> {
    let (_session, user) = require_auth_db(headers)
        .await
        .map_err(|response| *response)?;

    let target_pid = Uuid::parse_str(payload.user_id.trim()).map_err(|_error| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Invalid user ID".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        )
    })?;

    if target_pid == user.pid {
        return Err(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Cannot create DM with yourself".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| {
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        })?;

    let target_exists = users::table
        .filter(users::pid.eq(target_pid))
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| {
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        })?
        .is_some();

    if !target_exists {
        return Err(error_response(
            axum::http::StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: "Target user not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    Ok((user.pid, target_pid))
}

pub(super) fn dm_room_internal_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    )
}
