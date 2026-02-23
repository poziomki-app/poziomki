type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::super::state::{
    extract_bearer_token, resolve_session_by_token, session_model_to_view, user_model_to_view,
    SessionResponse,
};
use crate::db::models::sessions::Session;
use crate::db::models::users::User;
use crate::db::schema::users;

fn empty_session_response() -> Response {
    Json(SessionResponse {
        session: None,
        user: None,
    })
    .into_response()
}

async fn resolve_session_and_user(
    headers: &HeaderMap,
) -> std::result::Result<Option<(Session, User)>, crate::error::AppError> {
    let Some(token) = extract_bearer_token(headers) else {
        return Ok(None);
    };

    let session = resolve_session_by_token(&token).await?;

    let session = session.filter(|s| s.expires_at > Utc::now());
    let Some(session) = session else {
        return Ok(None);
    };

    let mut conn = crate::db::conn().await?;
    let user = users::table
        .find(session.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()?;

    Ok(user.map(|u| (session, u)))
}

pub(in crate::controllers::api) async fn get_session(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let resolved = resolve_session_and_user(&headers).await?;

    let Some((session, user)) = resolved else {
        return Ok(empty_session_response());
    };

    Ok(Json(SessionResponse {
        session: Some(session_model_to_view(&session)),
        user: Some(user_model_to_view(&user)),
    })
    .into_response())
}
