use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};

use super::super::state::{
    extract_bearer_token, resolve_session_by_token, session_model_to_view, user_model_to_view,
    SessionResponse,
};
use crate::models::_entities::{sessions, users};

fn empty_session_response() -> Response {
    Json(SessionResponse {
        session: None,
        user: None,
    })
    .into_response()
}

async fn resolve_session_and_user(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> std::result::Result<Option<(sessions::Model, users::Model)>, loco_rs::Error> {
    let Some(token) = extract_bearer_token(headers) else {
        return Ok(None);
    };

    let session = resolve_session_by_token(db, &token).await?;

    let session = session.filter(|s| s.expires_at.with_timezone(&Utc) > Utc::now());
    let Some(session) = session else {
        return Ok(None);
    };

    let user = users::Entity::find_by_id(session.user_id)
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    Ok(user.map(|u| (session, u)))
}

pub(in crate::controllers::migration_api) async fn get_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let resolved = resolve_session_and_user(&ctx.db, &headers).await?;

    let Some((session, user)) = resolved else {
        return Ok(empty_session_response());
    };

    Ok(Json(SessionResponse {
        session: Some(session_model_to_view(&session)),
        user: Some(user_model_to_view(&user)),
    })
    .into_response())
}
