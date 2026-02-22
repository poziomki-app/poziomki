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
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let target_pid = match Uuid::parse_str(payload.user_id.trim()) {
        Ok(pid) => pid,
        Err(_error) => {
            return Ok(error_response(
                axum::http::StatusCode::BAD_REQUEST,
                &headers,
                ErrorSpec {
                    error: "Invalid user ID".to_string(),
                    code: "BAD_REQUEST",
                    details: None,
                },
            ));
        }
    };

    if target_pid == user.pid {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Cannot create DM with yourself".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let target_exists = users::table
        .filter(users::pid.eq(target_pid))
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?
        .is_some();
    if !target_exists {
        return Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Target user not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    let room_id = match ensure_dm_room(&headers, user.pid, target_pid).await {
        Ok(room_id) => room_id,
        Err(response) => return Ok(response),
    };

    Ok(Json(DataResponse {
        data: MatrixRoomData { room_id },
    })
    .into_response())
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
