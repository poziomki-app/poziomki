use std::collections::HashSet;

use axum::http::HeaderMap;
use axum::response::Response;
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::super::{bootstrap_matrix_auth, matrix_support};
use crate::models::_entities::{event_attendees, profiles, users};

pub(super) async fn create_event_room(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    event_id: Uuid,
    event_title: &str,
    creator_profile_id: Uuid,
    requesting_user_pid: Uuid,
) -> std::result::Result<String, Response> {
    let bootstrap =
        bootstrap_matrix_auth(&requesting_user_pid.to_string(), headers, None, None).await?;
    let server_name = matrix_support::matrix_server_name_from_user_id(&bootstrap.auth.user_id)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            super::super::chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service returned an invalid user identifier",
                "CHAT_UNAVAILABLE",
            )
        })?;

    let attendee_user_pids = load_event_attendee_user_pids(db, event_id, creator_profile_id)
        .await
        .map_err(|e| {
            tracing::warn!(%e, "failed to load event attendees for room creation");
            super::event_room_internal_error(headers, "Failed to resolve event attendees")
        })?;

    let invites: Vec<String> = attendee_user_pids
        .iter()
        .map(|pid| matrix_support::matrix_user_id_from_pid(pid, &server_name))
        .filter(|user_id| user_id != &bootstrap.auth.user_id)
        .collect();

    let room_name = if event_title.trim().is_empty() {
        "Wydarzenie".to_string()
    } else {
        event_title.trim().to_string()
    };

    matrix_support::create_private_room(
        &bootstrap.http_client,
        &bootstrap.homeserver,
        &bootstrap.auth.access_token,
        &room_name,
        &invites,
        false,
    )
    .await
    .map_err(|error| {
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
    db: &DatabaseConnection,
    event_id: Uuid,
    creator_profile_id: Uuid,
    requester_profile_id: Uuid,
) -> std::result::Result<bool, crate::error::AppError> {
    if requester_profile_id == creator_profile_id {
        return Ok(true);
    }

    let attendee = event_attendees::Entity::find()
        .filter(event_attendees::Column::EventId.eq(event_id))
        .filter(event_attendees::Column::ProfileId.eq(requester_profile_id))
        .filter(event_attendees::Column::Status.eq("going"))
        .one(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(attendee.is_some())
}

async fn load_event_attendee_user_pids(
    db: &DatabaseConnection,
    event_id: Uuid,
    creator_profile_id: Uuid,
) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
    let attendee_rows = event_attendees::Entity::find()
        .filter(event_attendees::Column::EventId.eq(event_id))
        .filter(event_attendees::Column::Status.eq("going"))
        .all(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let mut profile_ids: HashSet<Uuid> = attendee_rows
        .into_iter()
        .map(|row| row.profile_id)
        .collect();
    profile_ids.insert(creator_profile_id);

    let profile_models = profiles::Entity::find()
        .filter(profiles::Column::Id.is_in(profile_ids))
        .all(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let user_ids: Vec<i32> = profile_models
        .into_iter()
        .map(|profile| profile.user_id)
        .collect();
    if user_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let user_models = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids))
        .all(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(user_models.into_iter().map(|user| user.pid).collect())
}
