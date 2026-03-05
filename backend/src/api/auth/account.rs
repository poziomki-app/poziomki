#[path = "export_queries.rs"]
mod auth_export_queries;

use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

use crate::api::auth_or_respond;
use super::super::state::{DataResponse, DeleteAccountBody, SuccessResponse};
use super::auth_service::unauthorized_error;
type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use crate::db::models::profiles::Profile;
use crate::db::schema::{profiles, sessions, users};

async fn delete_user_data(user_id: i32) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    conn.transaction(|conn| {
        Box::pin(async move {
            diesel::delete(profiles::table.filter(profiles::user_id.eq(user_id)))
                .execute(conn)
                .await?;
            diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                .execute(conn)
                .await?;
            diesel::delete(users::table.find(user_id))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        })
    })
    .await?;
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

    Ok(Json(SuccessResponse { success: true }).into_response())
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
            "age": p.age,
            "profilePicture": p.profile_picture,
            "images": p.images,
            "program": p.program,
            "createdAt": p.created_at.to_rfc3339(),
            "updatedAt": p.updated_at.to_rfc3339(),
        })
    });

    let (tags, created_events, attended_events, user_uploads) =
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
    ),
    crate::error::AppError,
> {
    let Some(pid) = profile_id else {
        return Ok((vec![], vec![], vec![], vec![]));
    };

    let tags = auth_export_queries::load_user_tags(pid).await?;
    let created = auth_export_queries::load_created_events(pid).await?;
    let attended = auth_export_queries::load_attended_events(pid).await?;
    let uploads = auth_export_queries::load_user_uploads(pid).await?;

    Ok((tags, created, attended, uploads))
}
