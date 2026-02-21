use std::time::Duration;

use axum::http::HeaderMap;
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::time::sleep;
use uuid::Uuid;

use super::super::{
    build_pending_token, is_matrix_room_id, EVENT_PENDING_RETRIES, PENDING_SLEEP_MS,
};
use super::{creation::create_event_room, EventRoomResolution};
use crate::models::_entities::events;

pub(super) async fn ensure_event_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event_id: Uuid,
    event_title: &str,
    creator_profile_id: Uuid,
    requesting_user_pid: Uuid,
) -> std::result::Result<EventRoomResolution, Response> {
    let mut pending_retries = 0usize;

    loop {
        let current_event = events::Entity::find_by_id(event_id)
            .one(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))
            .map_err(|_error| {
                super::event_room_internal_error(headers, "Failed to resolve event room")
            })?
            .ok_or_else(|| {
                super::super::super::error_response(
                    axum::http::StatusCode::NOT_FOUND,
                    headers,
                    super::super::super::ErrorSpec {
                        error: "Event not found".to_string(),
                        code: "NOT_FOUND",
                        details: None,
                    },
                )
            })?;

        if let Some(room_id) = current_event
            .conversation_id
            .as_deref()
            .filter(|value| is_matrix_room_id(value))
            .map(ToOwned::to_owned)
        {
            return Ok(EventRoomResolution {
                room_id,
                from_existing_mapping: true,
            });
        }

        if let Some(existing_pending) = current_event
            .conversation_id
            .as_deref()
            .filter(|value| value.starts_with(super::super::PENDING_PREFIX))
            .map(ToOwned::to_owned)
        {
            if pending_retries < EVENT_PENDING_RETRIES {
                pending_retries = pending_retries.saturating_add(1);
                sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
                continue;
            }

            let takeover_pending = build_pending_token();
            let took_over =
                claim_event_pending_token(db, event_id, Some(&existing_pending), &takeover_pending)
                    .await
                    .map_err(|e| loco_rs::Error::Any(e.into()))
                    .map_err(|_error| {
                        super::event_room_internal_error(headers, "Failed to resolve event room")
                    })?;

            if !took_over {
                pending_retries = 0;
                continue;
            }

            return create_and_finalize_event_room(
                db,
                headers,
                event_id,
                event_title,
                creator_profile_id,
                requesting_user_pid,
                &takeover_pending,
            )
            .await;
        }

        let pending_token = build_pending_token();
        let claimed = claim_event_pending_token(
            db,
            event_id,
            current_event.conversation_id.as_deref(),
            &pending_token,
        )
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
        .map_err(|_error| {
            super::event_room_internal_error(headers, "Failed to resolve event room")
        })?;

        if !claimed {
            pending_retries = pending_retries.saturating_add(1);
            sleep(Duration::from_millis(PENDING_SLEEP_MS)).await;
            continue;
        }

        return create_and_finalize_event_room(
            db,
            headers,
            event_id,
            event_title,
            creator_profile_id,
            requesting_user_pid,
            &pending_token,
        )
        .await;
    }
}

async fn create_and_finalize_event_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event_id: Uuid,
    event_title: &str,
    creator_profile_id: Uuid,
    requesting_user_pid: Uuid,
    pending_token: &str,
) -> std::result::Result<EventRoomResolution, Response> {
    let room_id_result = create_event_room(
        db,
        headers,
        event_id,
        event_title,
        creator_profile_id,
        requesting_user_pid,
    )
    .await;

    let room_id = match room_id_result {
        Ok(room_id) => room_id,
        Err(response) => {
            let _ = clear_event_pending_token(db, event_id, pending_token).await;
            return Err(response);
        }
    };

    let finalized = finalize_event_pending_token(db, event_id, pending_token, &room_id)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
        .map_err(|_error| {
            super::event_room_internal_error(headers, "Failed to finalize event room mapping")
        })?;

    if finalized {
        return Ok(EventRoomResolution {
            room_id,
            from_existing_mapping: false,
        });
    }

    let fallback_room_id = events::Entity::find_by_id(event_id)
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
        .map_err(|_error| {
            super::event_room_internal_error(headers, "Failed to resolve canonical event room")
        })?
        .and_then(|event| event.conversation_id)
        .filter(|value| is_matrix_room_id(value))
        .unwrap_or(room_id);

    Ok(EventRoomResolution {
        room_id: fallback_room_id,
        from_existing_mapping: false,
    })
}

async fn claim_event_pending_token(
    db: &DatabaseConnection,
    event_id: Uuid,
    expected_conversation_id: Option<&str>,
    pending_token: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let mut update = events::Entity::update_many()
        .col_expr(
            events::Column::ConversationId,
            Expr::value(pending_token.to_string()),
        )
        .col_expr(events::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(events::Column::Id.eq(event_id));

    if let Some(expected) = expected_conversation_id {
        update = update.filter(events::Column::ConversationId.eq(expected));
    } else {
        update = update.filter(events::Column::ConversationId.is_null());
    }

    let result = update.exec(db).await?;
    Ok(result.rows_affected == 1)
}

async fn finalize_event_pending_token(
    db: &DatabaseConnection,
    event_id: Uuid,
    pending_token: &str,
    room_id: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let result = events::Entity::update_many()
        .col_expr(
            events::Column::ConversationId,
            Expr::value(room_id.to_string()),
        )
        .col_expr(events::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(events::Column::Id.eq(event_id))
        .filter(events::Column::ConversationId.eq(pending_token))
        .exec(db)
        .await?;
    Ok(result.rows_affected == 1)
}

async fn clear_event_pending_token(
    db: &DatabaseConnection,
    event_id: Uuid,
    pending_token: &str,
) -> std::result::Result<bool, sea_orm::DbErr> {
    let result = events::Entity::update_many()
        .col_expr(
            events::Column::ConversationId,
            Expr::value(Option::<String>::None),
        )
        .col_expr(events::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(events::Column::Id.eq(event_id))
        .filter(events::Column::ConversationId.eq(pending_token))
        .exec(db)
        .await?;
    Ok(result.rows_affected == 1)
}
