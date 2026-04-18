type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};

use super::super::state::{
    extract_bearer_token, hash_session_token, session_model_to_view, user_model_to_view,
    SessionResponse,
};
use crate::db::models::sessions::Session;
use crate::db::models::users::User;
use crate::db::schema::users;
use crate::db::{self, DbViewer};

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
    let hashed = hash_session_token(&token);

    // Single transaction: SECURITY DEFINER session resolution, then viewer
    // context, then the User SELECT under that context. Matches the pattern
    // used by the require_auth_context middleware so both stay RLS-ready.
    let mut conn = crate::db::conn().await?;
    let result = conn
        .transaction::<Option<(Session, User)>, diesel::result::Error, _>(|conn| {
            async move {
                let Some(row) = db::resolve_session(conn, &hashed).await? else {
                    return Ok(None);
                };
                if row.expires_at <= Utc::now() {
                    return Ok(None);
                }
                db::set_viewer_context(
                    conn,
                    DbViewer {
                        user_id: row.user_id,
                        is_review_stub: row.is_review_stub,
                    },
                )
                .await?;
                let user = users::table
                    .find(row.user_id)
                    .first::<User>(conn)
                    .await
                    .optional()?;
                let session = Session {
                    id: row.session_id,
                    user_id: row.user_id,
                    token: row.token,
                    ip_address: row.ip_address,
                    user_agent: row.user_agent,
                    expires_at: row.expires_at,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                Ok(user.map(|u| (session, u)))
            }
            .scope_boxed()
        })
        .await?;
    Ok(result)
}

pub(in crate::api) async fn get_session(
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
