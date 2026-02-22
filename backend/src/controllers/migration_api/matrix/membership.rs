use axum::http::HeaderMap;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{bootstrap_matrix_auth, chat_bootstrap_error, is_matrix_room_id, matrix_support};
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::models::users::User;
use crate::db::schema::{profiles, users};

pub(super) async fn sync_event_membership_after_attend_result(
    headers: &HeaderMap,
    event: &Event,
    profile: &Profile,
) -> std::result::Result<(), String> {
    if let Some(room_id) = event
        .conversation_id
        .as_deref()
        .filter(|value| is_matrix_room_id(value))
    {
        ensure_profile_joined_room_best_effort(headers, event, profile, room_id).await?;
    }
    Ok(())
}

pub(super) async fn sync_event_membership_after_leave_result(
    headers: &HeaderMap,
    event: &Event,
    profile: &Profile,
) -> std::result::Result<(), String> {
    let Some(room_id) = event_room_id(event) else {
        return Ok(());
    };
    let Some(user) = load_leave_sync_user(profile).await else {
        return Ok(());
    };
    let bootstrap = bootstrap_leave_sync_user(headers, &user.pid)
        .await
        .ok_or_else(|| "matrix bootstrap unavailable during leave sync".to_string())?;

    matrix_support::leave_room(
        &bootstrap.http_client,
        &bootstrap.homeserver,
        &bootstrap.auth.access_token,
        room_id,
    )
    .await
    .map_err(|error| {
        format!(
            "leave room failed: status={} errcode={:?} message={} room_id={room_id}",
            error.status_code, error.errcode, error.message
        )
    })?;
    Ok(())
}

fn event_room_id(event: &Event) -> Option<&str> {
    event
        .conversation_id
        .as_deref()
        .filter(|value| is_matrix_room_id(value))
}

async fn load_leave_sync_user(profile: &Profile) -> Option<User> {
    let mut conn = match crate::db::conn().await {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(%error, "failed to get db connection for event leave sync");
            return None;
        }
    };
    match users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
    {
        Ok(user) => user,
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
    headers: &HeaderMap,
    event: &Event,
    profile: &Profile,
    room_id: &str,
) -> std::result::Result<(), axum::response::Response> {
    ensure_profile_joined_room_best_effort(headers, event, profile, room_id)
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
    headers: &HeaderMap,
    event: &Event,
    profile: &Profile,
    room_id: &str,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|e| e.to_string())?;

    let attendee_user = users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
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

        let creator_profile = profiles::table
            .find(event.creator_id)
            .first::<Profile>(&mut conn)
            .await
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Event creator profile not found".to_string())?;

        let creator_user = users::table
            .find(creator_profile.user_id)
            .first::<User>(&mut conn)
            .await
            .optional()
            .map_err(|e| e.to_string())?
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
