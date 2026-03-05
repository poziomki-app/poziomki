use std::collections::HashSet;

use axum::http::HeaderMap;
use axum::response::Response;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::super::{bootstrap_matrix_auth, matrix_service};
use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::profiles::Profile;
use crate::db::models::users::User;
use crate::db::schema::{event_attendees, profiles, users};

pub(super) struct EventRoomRequest {
    pub(super) event_id: Uuid,
    pub(super) event_title: String,
    pub(super) creator_profile_id: Uuid,
    pub(super) requesting_user_pid: Uuid,
}

pub(super) async fn create_event_room(
    headers: &HeaderMap,
    req: &EventRoomRequest,
) -> std::result::Result<String, Response> {
    let bootstrap =
        bootstrap_matrix_auth(&req.requesting_user_pid.to_string(), headers, None, None).await?;
    let server_name = matrix_service::matrix_server_name_from_user_id(&bootstrap.auth.user_id)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            super::super::chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service returned an invalid user identifier",
                "CHAT_UNAVAILABLE",
            )
        })?;

    let attendee_user_pids = load_event_attendee_user_pids(req.event_id, req.creator_profile_id)
        .await
        .map_err(|e| {
            tracing::warn!(%e, "failed to load event attendees for room creation");
            super::event_room_internal_error(headers, "Failed to resolve event attendees")
        })?;

    let invites: Vec<String> = attendee_user_pids
        .iter()
        .map(|pid| matrix_service::matrix_user_id_from_pid(pid, &server_name))
        .filter(|user_id| user_id != &bootstrap.auth.user_id)
        .collect();

    let room_name = if req.event_title.trim().is_empty() {
        "Wydarzenie".to_string()
    } else {
        req.event_title.trim().to_string()
    };

    let result = bootstrap
        .client()
        .create_private_room(Some(&room_name), &invites, false)
        .await;

    if let Some(m) = crate::metrics::metrics() {
        if result.is_ok() {
            m.matrix_room_create.inc_success();
        } else {
            m.matrix_room_create.inc_failure();
        }
    }

    result.map_err(|error| {
        tracing::warn!(
            status_code = error.status_code,
            errcode = error.errcode,
            message = error.message,
            "failed to create Matrix event room"
        );
        super::super::chat_bootstrap_error(
            axum::http::StatusCode::BAD_GATEWAY,
            headers,
            "Messaging service is temporarily unavailable",
            "CHAT_UNAVAILABLE",
        )
    })
}

pub(super) async fn can_access_event_chat(
    event_id: Uuid,
    creator_profile_id: Uuid,
    requester_profile_id: Uuid,
) -> std::result::Result<bool, crate::error::AppError> {
    if requester_profile_id == creator_profile_id {
        return Ok(true);
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let attendee = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::profile_id.eq(requester_profile_id))
        .filter(event_attendees::status.eq("going"))
        .first::<EventAttendee>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(attendee.is_some())
}

async fn load_event_attendee_user_pids(
    event_id: Uuid,
    creator_profile_id: Uuid,
) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let attendee_rows = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::status.eq("going"))
        .load::<EventAttendee>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let mut profile_ids: HashSet<Uuid> = attendee_rows
        .into_iter()
        .map(|row| row.profile_id)
        .collect();
    profile_ids.insert(creator_profile_id);

    let profile_models = profiles::table
        .filter(profiles::id.eq_any(&profile_ids.into_iter().collect::<Vec<_>>()))
        .load::<Profile>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let user_ids: Vec<i32> = profile_models
        .into_iter()
        .map(|profile| profile.user_id)
        .collect();
    if user_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let user_models = users::table
        .filter(users::id.eq_any(&user_ids))
        .load::<User>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(user_models.into_iter().map(|user| user.pid).collect())
}
