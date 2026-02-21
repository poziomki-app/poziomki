use axum::http::HeaderMap;
use sea_orm::{DatabaseConnection, EntityTrait};

use super::{bootstrap_matrix_auth, chat_bootstrap_error, is_matrix_room_id, matrix_support};
use crate::models::_entities::{events, profiles, users};

pub(super) async fn sync_event_membership_after_attend(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event: &events::Model,
    profile: &profiles::Model,
) {
    if let Some(room_id) = event
        .conversation_id
        .as_deref()
        .filter(|value| is_matrix_room_id(value))
    {
        if let Err(error) =
            ensure_profile_joined_room_best_effort(db, headers, event, profile, room_id).await
        {
            tracing::warn!(%error, event_id = %event.id, "failed to sync event-attend membership");
        }
    }
}

pub(super) async fn sync_event_membership_after_leave(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event: &events::Model,
    profile: &profiles::Model,
) {
    let Some(room_id) = event_room_id(event) else {
        return;
    };
    let Some(user) = load_leave_sync_user(db, profile).await else {
        return;
    };
    let Some(bootstrap) = bootstrap_leave_sync_user(headers, &user.pid).await else {
        return;
    };

    if let Err(error) = matrix_support::leave_room(
        &bootstrap.http_client,
        &bootstrap.homeserver,
        &bootstrap.auth.access_token,
        room_id,
    )
    .await
    {
        tracing::warn!(
            status_code = error.status_code,
            errcode = error.errcode,
            message = error.message,
            room_id,
            "failed to leave Matrix room after event leave"
        );
    }
}

fn event_room_id(event: &events::Model) -> Option<&str> {
    event
        .conversation_id
        .as_deref()
        .filter(|value| is_matrix_room_id(value))
}

async fn load_leave_sync_user(
    db: &DatabaseConnection,
    profile: &profiles::Model,
) -> Option<users::Model> {
    match users::Entity::find_by_id(profile.user_id).one(db).await {
        Ok(Some(user)) => Some(user),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(%error, "failed to load user for event leave sync");
            None
        }
    }
}

async fn bootstrap_leave_sync_user(
    headers: &HeaderMap,
    user_pid: &uuid::Uuid,
) -> Option<super::MatrixBootstrap> {
    match bootstrap_matrix_auth(&user_pid.to_string(), headers, None, None).await {
        Ok(bootstrap) => Some(bootstrap),
        Err(response) => {
            tracing::warn!(
                status = response.status().as_u16(),
                "matrix bootstrap unavailable during leave sync"
            );
            None
        }
    }
}

pub(super) async fn ensure_profile_joined_event_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event: &events::Model,
    profile: &profiles::Model,
    room_id: &str,
) -> std::result::Result<(), loco_rs::prelude::Response> {
    ensure_profile_joined_room_best_effort(db, headers, event, profile, room_id)
        .await
        .map_err(|error| {
            tracing::warn!(%error, event_id = %event.id, "event room join failed");
            chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
            )
        })
}

async fn ensure_profile_joined_room_best_effort(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event: &events::Model,
    profile: &profiles::Model,
    room_id: &str,
) -> std::result::Result<(), String> {
    let attendee_user = users::Entity::find_by_id(profile.user_id)
        .one(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "User not found for attendee profile".to_string())?;

    let attendee_bootstrap =
        bootstrap_matrix_auth(&attendee_user.pid.to_string(), headers, None, None)
            .await
            .map_err(|response| {
                format!("matrix bootstrap failed with status {}", response.status())
            })?;

    if let Err(error) = matrix_support::join_room(
        &attendee_bootstrap.http_client,
        &attendee_bootstrap.homeserver,
        &attendee_bootstrap.auth.access_token,
        room_id,
    )
    .await
    {
        let forbidden_error = error.status_code == 403;
        if !forbidden_error {
            return Err(format!(
                "join room failed: status={} errcode={:?} message={}",
                error.status_code, error.errcode, error.message
            ));
        }

        let creator_profile = profiles::Entity::find_by_id(event.creator_id)
            .one(db)
            .await
            .map_err(|db_error| db_error.to_string())?
            .ok_or_else(|| "Event creator profile not found".to_string())?;

        let creator_user = users::Entity::find_by_id(creator_profile.user_id)
            .one(db)
            .await
            .map_err(|db_error| db_error.to_string())?
            .ok_or_else(|| "Event creator user not found".to_string())?;

        let creator_bootstrap =
            bootstrap_matrix_auth(&creator_user.pid.to_string(), headers, None, None)
                .await
                .map_err(|response| {
                    format!(
                        "creator matrix bootstrap failed with status {}",
                        response.status()
                    )
                })?;

        matrix_support::invite_user_to_room(
            &creator_bootstrap.http_client,
            &creator_bootstrap.homeserver,
            &creator_bootstrap.auth.access_token,
            room_id,
            &attendee_bootstrap.auth.user_id,
        )
        .await
        .map_err(|invite_error| {
            format!(
                "invite failed: status={} errcode={:?} message={}",
                invite_error.status_code, invite_error.errcode, invite_error.message
            )
        })?;

        matrix_support::join_room(
            &attendee_bootstrap.http_client,
            &attendee_bootstrap.homeserver,
            &attendee_bootstrap.auth.access_token,
            room_id,
        )
        .await
        .map_err(|join_error| {
            format!(
                "join after invite failed: status={} errcode={:?} message={}",
                join_error.status_code, join_error.errcode, join_error.message
            )
        })?;
    }

    Ok(())
}
