use std::time::Duration;

use axum::http::HeaderMap;
use axum::response::Response;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::time::sleep;
use uuid::Uuid;

use super::super::{
    build_pending_token, is_matrix_room_id, DM_PENDING_RETRIES, PENDING_PREFIX, PENDING_SLEEP_MS,
};
use super::creation::create_dm_room;
use crate::db::models::matrix_dm_rooms::{MatrixDmRoom, NewMatrixDmRoom};
use crate::db::schema::matrix_dm_rooms;

#[allow(clippy::struct_field_names)]
struct DmRoomContext {
    own_user_pid: Uuid,
    other_user_pid: Uuid,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
}

pub(super) async fn ensure_dm_room(
    headers: &HeaderMap,
    own_user_pid: Uuid,
    other_user_pid: Uuid,
) -> std::result::Result<String, Response> {
    let (user_low_pid, user_high_pid) = canonical_user_pair(own_user_pid, other_user_pid);
    let ctx = DmRoomContext {
        own_user_pid,
        other_user_pid,
        user_low_pid,
        user_high_pid,
    };
    let mut pending_retries = 0usize;

    loop {
        match try_resolve_dm_room_iteration(headers, &ctx, &mut pending_retries).await? {
            DmLoopResult::Resolved(room_id) => return Ok(room_id),
            DmLoopResult::Retry => {}
            DmLoopResult::Create(pending_token) => {
                return create_and_finalize_dm_room(headers, &ctx, &pending_token).await;
            }
        }
    }
}

enum DmLoopResult {
    Resolved(String),
    Retry,
    Create(String),
}

async fn try_resolve_dm_room_iteration(
    headers: &HeaderMap,
    ctx: &DmRoomContext,
    pending_retries: &mut usize,
) -> std::result::Result<DmLoopResult, Response> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| super::dm_room_internal_error(headers, "Failed to resolve DM room"))?;

    let existing = matrix_dm_rooms::table
        .filter(matrix_dm_rooms::user_low_pid.eq(ctx.user_low_pid))
        .filter(matrix_dm_rooms::user_high_pid.eq(ctx.user_high_pid))
        .first::<MatrixDmRoom>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| super::dm_room_internal_error(headers, "Failed to resolve DM room"))?;

    if let Some(record) = existing {
        return resolve_existing_dm_record(headers, ctx, &record, pending_retries).await;
    }

    let pending_token = build_pending_token();
    let inserted = insert_dm_pending_row(ctx.user_low_pid, ctx.user_high_pid, &pending_token)
        .await
        .map_err(|_error| {
            super::dm_room_internal_error(headers, "Failed to reserve DM room mapping")
        })?;

    if inserted {
        return Ok(DmLoopResult::Create(pending_token));
    }

    *pending_retries = pending_retries.saturating_add(1);
    sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
    Ok(DmLoopResult::Retry)
}

async fn resolve_existing_dm_record(
    headers: &HeaderMap,
    ctx: &DmRoomContext,
    record: &MatrixDmRoom,
    pending_retries: &mut usize,
) -> std::result::Result<DmLoopResult, Response> {
    if is_matrix_room_id(&record.room_id) {
        return Ok(DmLoopResult::Resolved(record.room_id.clone()));
    }

    if record.room_id.starts_with(PENDING_PREFIX) && *pending_retries < DM_PENDING_RETRIES {
        *pending_retries = pending_retries.saturating_add(1);
        sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
        return Ok(DmLoopResult::Retry);
    }

    try_dm_takeover(headers, ctx, &record.room_id, pending_retries).await
}

async fn try_dm_takeover(
    headers: &HeaderMap,
    ctx: &DmRoomContext,
    existing_room_id: &str,
    pending_retries: &mut usize,
) -> std::result::Result<DmLoopResult, Response> {
    let takeover_pending = build_pending_token();
    let took_over = claim_dm_pending_token(
        ctx.user_low_pid,
        ctx.user_high_pid,
        existing_room_id,
        &takeover_pending,
    )
    .await
    .map_err(|_error| super::dm_room_internal_error(headers, "Failed to resolve DM room"))?;

    if took_over {
        Ok(DmLoopResult::Create(takeover_pending))
    } else {
        *pending_retries = 0;
        Ok(DmLoopResult::Retry)
    }
}

async fn create_and_finalize_dm_room(
    headers: &HeaderMap,
    ctx: &DmRoomContext,
    pending_token: &str,
) -> std::result::Result<String, Response> {
    let room_id = match create_dm_room(headers, ctx.own_user_pid, ctx.other_user_pid).await {
        Ok(room_id) => room_id,
        Err(response) => {
            let _ = delete_dm_pending_row(ctx.user_low_pid, ctx.user_high_pid, pending_token).await;
            return Err(response);
        }
    };

    let finalized =
        finalize_dm_pending_token(ctx.user_low_pid, ctx.user_high_pid, pending_token, &room_id)
            .await
            .map_err(|_error| {
                super::dm_room_internal_error(headers, "Failed to finalize DM room mapping")
            })?;

    if finalized {
        return Ok(room_id);
    }

    resolve_canonical_dm_room(headers, ctx, room_id).await
}

async fn resolve_canonical_dm_room(
    headers: &HeaderMap,
    ctx: &DmRoomContext,
    fallback_room_id: String,
) -> std::result::Result<String, Response> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| {
            super::dm_room_internal_error(headers, "Failed to resolve canonical DM room")
        })?;

    let canonical = matrix_dm_rooms::table
        .filter(matrix_dm_rooms::user_low_pid.eq(ctx.user_low_pid))
        .filter(matrix_dm_rooms::user_high_pid.eq(ctx.user_high_pid))
        .first::<MatrixDmRoom>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| {
            super::dm_room_internal_error(headers, "Failed to resolve canonical DM room")
        })?
        .map(|row| row.room_id)
        .filter(|value| is_matrix_room_id(value))
        .unwrap_or(fallback_room_id);

    Ok(canonical)
}

async fn claim_dm_pending_token(
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    expected_room_id: &str,
    pending_token: &str,
) -> std::result::Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let rows_affected = diesel::update(
        matrix_dm_rooms::table
            .filter(matrix_dm_rooms::user_low_pid.eq(user_low_pid))
            .filter(matrix_dm_rooms::user_high_pid.eq(user_high_pid))
            .filter(matrix_dm_rooms::room_id.eq(expected_room_id)),
    )
    .set((
        matrix_dm_rooms::room_id.eq(pending_token.to_string()),
        matrix_dm_rooms::updated_at.eq(Utc::now()),
    ))
    .execute(&mut conn)
    .await
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    Ok(rows_affected == 1)
}

async fn finalize_dm_pending_token(
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
    room_id: &str,
) -> std::result::Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let rows_affected = diesel::update(
        matrix_dm_rooms::table
            .filter(matrix_dm_rooms::user_low_pid.eq(user_low_pid))
            .filter(matrix_dm_rooms::user_high_pid.eq(user_high_pid))
            .filter(matrix_dm_rooms::room_id.eq(pending_token)),
    )
    .set((
        matrix_dm_rooms::room_id.eq(room_id.to_string()),
        matrix_dm_rooms::updated_at.eq(Utc::now()),
    ))
    .execute(&mut conn)
    .await
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    Ok(rows_affected == 1)
}

async fn delete_dm_pending_row(
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let rows_affected = diesel::delete(
        matrix_dm_rooms::table
            .filter(matrix_dm_rooms::user_low_pid.eq(user_low_pid))
            .filter(matrix_dm_rooms::user_high_pid.eq(user_high_pid))
            .filter(matrix_dm_rooms::room_id.eq(pending_token)),
    )
    .execute(&mut conn)
    .await
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    Ok(rows_affected == 1)
}

async fn insert_dm_pending_row(
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<bool, crate::error::AppError> {
    let now = Utc::now();
    let new = NewMatrixDmRoom {
        id: Uuid::new_v4(),
        user_low_pid,
        user_high_pid,
        room_id: pending_token.to_string(),
        created_at: now,
        updated_at: now,
    };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    match diesel::insert_into(matrix_dm_rooms::table)
        .values(&new)
        .execute(&mut conn)
        .await
    {
        Ok(_) => Ok(true),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => Ok(false),
        Err(error) => Err(crate::error::AppError::Any(error.into())),
    }
}

fn canonical_user_pair(first: Uuid, second: Uuid) -> (Uuid, Uuid) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}
