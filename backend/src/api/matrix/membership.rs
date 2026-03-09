use axum::http::HeaderMap;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{bootstrap_matrix_auth, chat_bootstrap_error, is_matrix_room_id};
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
        let result = ensure_profile_joined_room_best_effort(headers, event, profile, room_id).await;
        if let Some(m) = crate::metrics::metrics() {
            if result.is_ok() {
                m.matrix_membership.inc_success();
            } else {
                m.matrix_membership.inc_failure();
            }
        }
        result?;
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

    let result = bootstrap.client().leave_room(room_id).await;
    if let Some(m) = crate::metrics::metrics() {
        if result.is_ok() {
            m.matrix_membership.inc_success();
        } else {
            m.matrix_membership.inc_failure();
        }
    }
    result.map_err(|error| {
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
    let attendee_bootstrap = bootstrap_profile_user(headers, profile).await?;

    match attendee_bootstrap.client().join_room(room_id).await {
        Ok(()) => Ok(()),
        Err(error) if error.status_code == 403 => {
            invite_then_join(headers, event, &attendee_bootstrap, room_id).await
        }
        Err(error) => Err(format!(
            "join room failed: status={} errcode={:?} message={}",
            error.status_code, error.errcode, error.message
        )),
    }
}

async fn bootstrap_profile_user(
    headers: &HeaderMap,
    profile: &Profile,
) -> std::result::Result<super::MatrixBootstrap, String> {
    let mut conn = crate::db::conn().await.map_err(|e| e.to_string())?;
    let user = users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "User not found for profile".to_string())?;
    bootstrap_matrix_auth(&user.pid.to_string(), headers, None, None)
        .await
        .map_err(|response| format!("matrix bootstrap failed with status {}", response.status()))
}

async fn invite_then_join(
    headers: &HeaderMap,
    event: &Event,
    attendee_bootstrap: &super::MatrixBootstrap,
    room_id: &str,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|e| e.to_string())?;

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

    creator_bootstrap
        .client()
        .invite_user_to_room(room_id, &attendee_bootstrap.auth.user_id)
        .await
        .map_err(|invite_error| {
            format!(
                "invite failed: status={} errcode={:?} message={}",
                invite_error.status_code, invite_error.errcode, invite_error.message
            )
        })?;

    attendee_bootstrap
        .client()
        .join_room(room_id)
        .await
        .map_err(|join_error| {
            format!(
                "join after invite failed: status={} errcode={:?} message={}",
                join_error.status_code, join_error.errcode, join_error.message
            )
        })
}
