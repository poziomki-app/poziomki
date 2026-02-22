use std::time::Duration;

use axum::http::HeaderMap;
use axum::response::Response;
use chrono::Utc;
use sea_orm::{
    sea_query::Expr, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};
use tokio::time::sleep;
use uuid::Uuid;

use super::super::{
    build_pending_token, is_matrix_room_id, DM_PENDING_RETRIES, PENDING_PREFIX, PENDING_SLEEP_MS,
};
use super::creation::create_dm_room;
use crate::models::_entities::matrix_dm_rooms;

pub(super) async fn ensure_dm_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    own_user_pid: Uuid,
    other_user_pid: Uuid,
) -> std::result::Result<String, Response> {
    let (user_low_pid, user_high_pid) = canonical_user_pair(own_user_pid, other_user_pid);
    let mut pending_retries = 0usize;

    loop {
        let existing = matrix_dm_rooms::Entity::find()
            .filter(matrix_dm_rooms::Column::UserLowPid.eq(user_low_pid))
            .filter(matrix_dm_rooms::Column::UserHighPid.eq(user_high_pid))
            .one(db)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))
            .map_err(|_error| {
                super::dm_room_internal_error(headers, "Failed to resolve DM room")
            })?;

        if let Some(record) = existing {
            if is_matrix_room_id(&record.room_id) {
                return Ok(record.room_id);
            }

            if record.room_id.starts_with(PENDING_PREFIX) && pending_retries < DM_PENDING_RETRIES {
                pending_retries = pending_retries.saturating_add(1);
                sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
                continue;
            }

            let takeover_pending = build_pending_token();
            let took_over = claim_dm_pending_token(
                db,
                user_low_pid,
                user_high_pid,
                &record.room_id,
                &takeover_pending,
            )
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))
            .map_err(|_error| {
                super::dm_room_internal_error(headers, "Failed to resolve DM room")
            })?;

            if took_over {
                return create_and_finalize_dm_room(
                    db,
                    headers,
                    own_user_pid,
                    other_user_pid,
                    user_low_pid,
                    user_high_pid,
                    &takeover_pending,
                )
                .await;
            }

            pending_retries = 0;
            continue;
        }

        let pending_token = build_pending_token();
        let inserted = insert_dm_pending_row(db, user_low_pid, user_high_pid, &pending_token)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))
            .map_err(|_error| {
                super::dm_room_internal_error(headers, "Failed to reserve DM room mapping")
            })?;

        if inserted {
            return create_and_finalize_dm_room(
                db,
                headers,
                own_user_pid,
                other_user_pid,
                user_low_pid,
                user_high_pid,
                &pending_token,
            )
            .await;
        }

        pending_retries = pending_retries.saturating_add(1);
        sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_and_finalize_dm_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    own_user_pid: Uuid,
    other_user_pid: Uuid,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<String, Response> {
    let room_id_result = create_dm_room(headers, own_user_pid, other_user_pid).await;

    let room_id = match room_id_result {
        Ok(room_id) => room_id,
        Err(response) => {
            let _ = delete_dm_pending_row(db, user_low_pid, user_high_pid, pending_token).await;
            return Err(response);
        }
    };

    let finalized =
        finalize_dm_pending_token(db, user_low_pid, user_high_pid, pending_token, &room_id)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))
            .map_err(|_error| {
                super::dm_room_internal_error(headers, "Failed to finalize DM room mapping")
            })?;

    if finalized {
        return Ok(room_id);
    }

    let fallback_room_id = matrix_dm_rooms::Entity::find()
        .filter(matrix_dm_rooms::Column::UserLowPid.eq(user_low_pid))
        .filter(matrix_dm_rooms::Column::UserHighPid.eq(user_high_pid))
        .one(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
        .map_err(|_error| {
            super::dm_room_internal_error(headers, "Failed to resolve canonical DM room")
        })?
        .map(|row| row.room_id)
        .filter(|value| is_matrix_room_id(value))
        .unwrap_or(room_id);

    Ok(fallback_room_id)
}

async fn claim_dm_pending_token(
    db: &DatabaseConnection,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    expected_room_id: &str,
    pending_token: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let result = matrix_dm_rooms::Entity::update_many()
        .col_expr(
            matrix_dm_rooms::Column::RoomId,
            Expr::value(pending_token.to_string()),
        )
        .col_expr(matrix_dm_rooms::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(matrix_dm_rooms::Column::UserLowPid.eq(user_low_pid))
        .filter(matrix_dm_rooms::Column::UserHighPid.eq(user_high_pid))
        .filter(matrix_dm_rooms::Column::RoomId.eq(expected_room_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected == 1)
}

async fn finalize_dm_pending_token(
    db: &DatabaseConnection,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
    room_id: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let result = matrix_dm_rooms::Entity::update_many()
        .col_expr(
            matrix_dm_rooms::Column::RoomId,
            Expr::value(room_id.to_string()),
        )
        .col_expr(matrix_dm_rooms::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(matrix_dm_rooms::Column::UserLowPid.eq(user_low_pid))
        .filter(matrix_dm_rooms::Column::UserHighPid.eq(user_high_pid))
        .filter(matrix_dm_rooms::Column::RoomId.eq(pending_token))
        .exec(db)
        .await?;
    Ok(result.rows_affected == 1)
}

async fn delete_dm_pending_row(
    db: &DatabaseConnection,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let result = matrix_dm_rooms::Entity::delete_many()
        .filter(matrix_dm_rooms::Column::UserLowPid.eq(user_low_pid))
        .filter(matrix_dm_rooms::Column::UserHighPid.eq(user_high_pid))
        .filter(matrix_dm_rooms::Column::RoomId.eq(pending_token))
        .exec(db)
        .await?;
    Ok(result.rows_affected == 1)
}

async fn insert_dm_pending_row(
    db: &DatabaseConnection,
    user_low_pid: Uuid,
    user_high_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let now = Utc::now();
    let active = matrix_dm_rooms::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_low_pid: ActiveValue::Set(user_low_pid),
        user_high_pid: ActiveValue::Set(user_high_pid),
        room_id: ActiveValue::Set(pending_token.to_string()),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };

    match active.insert(db).await {
        Ok(_row) => Ok(true),
        Err(error) if is_unique_constraint_error(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_unique_constraint_error(error: &sea_orm::DbErr) -> bool {
    let text = error.to_string();
    text.contains("duplicate key") || text.contains("UNIQUE constraint failed")
}

fn canonical_user_pair(first: Uuid, second: Uuid) -> (Uuid, Uuid) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}
