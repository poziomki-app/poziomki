use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{app::AppContext, hash, prelude::*};

use super::super::{
    error_response,
    state::{require_auth_db, DataResponse, DeleteAccountBody, SuccessResponse},
    ErrorSpec,
};
use crate::models::_entities::{profiles, sessions, users};

fn unauthorized_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::UNAUTHORIZED,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "UNAUTHORIZED",
            details: None,
        },
    )
}

async fn delete_user_data(
    db: &DatabaseConnection,
    user_id: i32,
) -> std::result::Result<(), loco_rs::Error> {
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

    if payload.password.is_empty() || !hash::verify_password(&payload.password, &user.password) {
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
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

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

    let export = serde_json::json!({
        "user": {
            "id": user.pid.to_string(),
            "email": user.email,
            "name": user.name,
            "emailVerified": user.email_verified_at.is_some(),
            "createdAt": user.created_at.to_rfc3339(),
        },
        "profile": profile_view,
        "tags": [],
        "events": [],
        "eventsAttended": [],
        "conversations": [],
        "messages": [],
        "sessions": [],
        "exportedAt": Utc::now().to_rfc3339(),
    });

    Ok(Json(DataResponse { data: export }).into_response())
}
