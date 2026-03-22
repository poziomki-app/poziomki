#[path = "export_queries.rs"]
mod auth_export_queries;

use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

use crate::api::{auth_or_respond, error_response};

use super::super::state::{
    invalidate_auth_cache_for_user_id, ChangePasswordBody, DataResponse, DeleteAccountBody,
    SuccessResponse,
};
use super::auth_service::unauthorized_error;
type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use crate::db::models::profiles::Profile;
use crate::db::schema::{
    conversations, event_attendees, events, profile_tags, profiles, sessions, uploads,
    user_settings, users,
};

async fn delete_user_data(user_id: i32) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let profile_id: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .select(profiles::id)
        .first(&mut conn)
        .await
        .optional()?;

    // Collect upload filenames before the transaction for S3 cleanup afterwards
    let upload_filenames: Vec<String> = if let Some(pid) = profile_id {
        uploads::table
            .filter(uploads::owner_id.eq(pid))
            .select(uploads::filename)
            .load(&mut conn)
            .await?
    } else {
        vec![]
    };

    conn.transaction(|conn| {
        Box::pin(async move {
            if let Some(pid) = profile_id {
                // Collect upload IDs before removing the FK reference
                let upload_ids: Vec<uuid::Uuid> = uploads::table
                    .filter(uploads::owner_id.eq(pid))
                    .select(uploads::id)
                    .load(conn)
                    .await?;

                // Break uploads → profiles FK (owner_id is nullable)
                diesel::update(uploads::table.filter(uploads::owner_id.eq(pid)))
                    .set(uploads::owner_id.eq(None::<uuid::Uuid>))
                    .execute(conn)
                    .await?;

                // Remove user from events they attend (no CASCADE on profile_id)
                diesel::delete(event_attendees::table.filter(event_attendees::profile_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Remove profile tags (no CASCADE)
                diesel::delete(profile_tags::table.filter(profile_tags::profile_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Delete DM conversations (cascades → conv_members, messages, reactions)
                diesel::delete(
                    conversations::table
                        .filter(conversations::kind.eq("dm"))
                        .filter(
                            conversations::user_low_id
                                .eq(user_id)
                                .or(conversations::user_high_id.eq(user_id)),
                        ),
                )
                .execute(conn)
                .await?;

                // Delete user's events (cascades → attendees, tags, interactions,
                // feedback, event conversations + their messages)
                diesel::delete(events::table.filter(events::creator_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Delete user settings (no CASCADE)
                diesel::delete(user_settings::table.filter(user_settings::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete profile (cascades → event_interactions, recommendation_feedback, reports)
                diesel::delete(profiles::table.filter(profiles::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete sessions
                diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete user (cascades → conv_members, messages, reactions, push_subscriptions)
                diesel::delete(users::table.find(user_id))
                    .execute(conn)
                    .await?;

                // Clean up orphaned upload rows (safe: all message FK refs are gone)
                if !upload_ids.is_empty() {
                    diesel::delete(uploads::table.filter(uploads::id.eq_any(&upload_ids)))
                        .execute(conn)
                        .await?;
                }
            } else {
                // No profile — clean up user-level data only
                diesel::delete(
                    conversations::table
                        .filter(conversations::kind.eq("dm"))
                        .filter(
                            conversations::user_low_id
                                .eq(user_id)
                                .or(conversations::user_high_id.eq(user_id)),
                        ),
                )
                .execute(conn)
                .await?;

                diesel::delete(user_settings::table.filter(user_settings::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                diesel::delete(users::table.find(user_id))
                    .execute(conn)
                    .await?;
            }

            Ok::<(), diesel::result::Error>(())
        })
    })
    .await?;

    // Best-effort S3 cleanup (outside transaction — external side-effect)
    for filename in &upload_filenames {
        crate::api::uploads::delete_upload_objects(filename).await;
    }

    Ok(())
}

pub(in crate::api) async fn delete_account(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DeleteAccountBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if payload.password.is_empty()
        || !crate::security::verify_password(&payload.password, &user.password)
    {
        return Ok(unauthorized_error(&headers, "Invalid password"));
    }

    delete_user_data(user.id).await?;
    invalidate_auth_cache_for_user_id(user.id).await;

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(in crate::api) async fn change_password(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if payload.current_password.is_empty()
        || !crate::security::verify_password(&payload.current_password, &user.password)
    {
        return Ok(unauthorized_error(&headers, "Invalid password"));
    }

    if !(8..=128).contains(&payload.new_password.len()) {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            crate::api::ErrorSpec {
                error: "Password must be between 8 and 128 characters".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let new_hash = crate::security::hash_password(&payload.new_password)?;
    let mut conn = crate::db::conn().await?;

    conn.transaction(|conn| {
        let new_hash = new_hash.clone();
        Box::pin(async move {
            diesel::update(users::table.find(user.id))
                .set((
                    users::password.eq(new_hash),
                    users::updated_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;
            diesel::delete(sessions::table.filter(sessions::user_id.eq(user.id)))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        })
    })
    .await?;

    invalidate_auth_cache_for_user_id(user.id).await;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn export_data(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let mut conn = crate::db::conn().await?;

    let profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()?;

    let profile_id = profile.as_ref().map(|p| p.id);

    let profile_view: Option<serde_json::Value> = profile.map(|p| {
        serde_json::json!({
            "id": p.id.to_string(),
            "userId": user.pid.to_string(),
            "name": p.name,
            "bio": p.bio,
            "profilePicture": p.profile_picture,
            "images": p.images,
            "program": p.program,
            "createdAt": p.created_at.to_rfc3339(),
            "updatedAt": p.updated_at.to_rfc3339(),
        })
    });

    let (tags, created_events, attended_events, event_interactions, user_uploads, rec_feedback) =
        load_profile_data(profile_id).await?;

    let user_sessions = auth_export_queries::load_user_sessions(user.id).await?;
    let settings = auth_export_queries::load_user_settings(user.id).await?;

    let export = serde_json::json!({
        "user": {
            "id": user.pid.to_string(),
            "email": user.email,
            "name": user.name,
            "emailVerified": user.email_verified_at.is_some(),
            "createdAt": user.created_at.to_rfc3339(),
        },
        "profile": profile_view,
        "tags": tags,
        "events": created_events,
        "eventsAttended": attended_events,
        "eventInteractions": event_interactions,
        "recommendationFeedback": rec_feedback,
        "uploads": user_uploads,
        "sessions": user_sessions,
        "settings": settings,
        "conversations": [],
        "messages": [],
        "exportedAt": Utc::now().to_rfc3339(),
    });

    Ok(Json(DataResponse { data: export }).into_response())
}

async fn load_profile_data(
    profile_id: Option<uuid::Uuid>,
) -> std::result::Result<
    (
        Vec<serde_json::Value>,
        Vec<serde_json::Value>,
        Vec<serde_json::Value>,
        Vec<serde_json::Value>,
        Vec<serde_json::Value>,
        Vec<serde_json::Value>,
    ),
    crate::error::AppError,
> {
    let Some(pid) = profile_id else {
        return Ok((vec![], vec![], vec![], vec![], vec![], vec![]));
    };

    let tags = auth_export_queries::load_user_tags(pid).await?;
    let created = auth_export_queries::load_created_events(pid).await?;
    let attended = auth_export_queries::load_attended_events(pid).await?;
    let interactions = auth_export_queries::load_event_interactions(pid).await?;
    let uploads = auth_export_queries::load_user_uploads(pid).await?;
    let rec_feedback = auth_export_queries::load_recommendation_feedback(pid).await?;

    Ok((tags, created, attended, interactions, uploads, rec_feedback))
}
