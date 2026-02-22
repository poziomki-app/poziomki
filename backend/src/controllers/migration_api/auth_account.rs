#[path = "auth_export_queries.rs"]
mod auth_export_queries;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;

use super::super::state::{require_auth_db, DataResponse, DeleteAccountBody, SuccessResponse};
use super::auth_helpers::unauthorized_error;
use crate::models::_entities::{profiles, sessions, users};
use crate::security;
use sea_orm::DatabaseConnection;
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};

type Result<T> = crate::error::AppResult<T>;

async fn delete_user_data(
    db: &DatabaseConnection,
    user_id: i32,
) -> std::result::Result<(), crate::error::AppError> {
    let _ = profiles::Entity::delete_many()
        .filter(profiles::Column::UserId.eq(user_id))
        .exec(db)
        .await;
    let _ = sessions::Entity::delete_many()
        .filter(sessions::Column::UserId.eq(user_id))
        .exec(db)
        .await;
    let _ = users::Entity::delete_by_id(user_id).exec(db).await;
    Ok(())
}

pub(in crate::controllers::migration_api) async fn delete_account(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DeleteAccountBody>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    if payload.password.is_empty() || !security::verify_password(&payload.password, &user.password)
    {
        return Ok(unauthorized_error(&headers, "Invalid password"));
    }

    delete_user_data(&ctx.db, user.id).await?;

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(in crate::controllers::migration_api) async fn export_data(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

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
        load_profile_data(&ctx.db, profile_id).await?;

    let user_sessions = auth_export_queries::load_user_sessions(&ctx.db, user.id).await?;
    let settings = auth_export_queries::load_user_settings(&ctx.db, user.id).await?;

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
    db: &DatabaseConnection,
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

    let tags = auth_export_queries::load_user_tags(db, pid).await?;
    let created = auth_export_queries::load_created_events(db, pid).await?;
    let attended = auth_export_queries::load_attended_events(db, pid).await?;
    let uploads = auth_export_queries::load_user_uploads(db, pid).await?;

    Ok((tags, created, attended, uploads))
}
