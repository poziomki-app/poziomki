use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use uuid::Uuid;

use super::super::{error_response, ErrorSpec};
use super::{SessionView, UserView};
use crate::db::models::sessions::{NewSession, Session, SessionUpdate};
use crate::db::models::users::User;
use crate::db::schema::{sessions, users};

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;
const SESSION_TOKEN_HASH_PREFIX: &str = "st2:";

pub(in crate::api) fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let header = headers.get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

#[derive(Clone, Debug)]
pub(in crate::api) struct CreatedSession {
    pub(in crate::api) model: Session,
    pub(in crate::api) token: String,
}

fn session_token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let mut digest_hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut digest_hex, "{byte:02x}");
    }
    format!("{SESSION_TOKEN_HASH_PREFIX}{digest_hex}")
}

pub(in crate::api) fn hash_session_token(token: &str) -> String {
    session_token_hash(token)
}

pub(in crate::api) async fn resolve_session_by_token(
    token: &str,
) -> std::result::Result<Option<Session>, crate::error::AppError> {
    let hashed = session_token_hash(token);
    let mut conn = crate::db::conn().await?;
    Ok(sessions::table
        .filter(sessions::token.eq(&hashed))
        .first::<Session>(&mut conn)
        .await
        .optional()?)
}

pub(in crate::api) async fn require_auth_db(
    headers: &HeaderMap,
) -> std::result::Result<(Session, User), Box<axum::response::Response>> {
    let token =
        extract_bearer_token(headers).ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let session = resolve_session_by_token(&token)
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    delete_if_expired(&session, headers).await?;
    maybe_renew_session(&session).await;

    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?;
    let user = users::table
        .find(session.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    Ok((session, user))
}

async fn delete_if_expired(
    session: &Session,
    headers: &HeaderMap,
) -> std::result::Result<(), Box<axum::response::Response>> {
    let now = Utc::now();
    if session.expires_at <= now {
        if let Ok(mut conn) = crate::db::conn().await {
            let _ = diesel::delete(sessions::table.find(session.id))
                .execute(&mut conn)
                .await;
        }
        return Err(Box::new(unauthorized_response(headers)));
    }
    Ok(())
}

async fn maybe_renew_session(session: &Session) {
    let now = Utc::now();
    let elapsed = now - session.updated_at;
    if elapsed >= Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        let new_expires = now + Duration::seconds(SESSION_DURATION_SECS);
        if let Ok(mut conn) = crate::db::conn().await {
            let _ = diesel::update(sessions::table.find(session.id))
                .set(&SessionUpdate {
                    updated_at: Some(now),
                    expires_at: Some(new_expires),
                })
                .execute(&mut conn)
                .await;
        }
    }
}

pub(in crate::api) async fn create_session_db(
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<CreatedSession, crate::error::AppError> {
    let now = Utc::now();
    let session_id = Uuid::new_v4();
    let secret = Uuid::new_v4().simple().to_string();
    let token = format!("{session_id}.{secret}");
    let new = NewSession {
        id: session_id,
        user_id,
        token: session_token_hash(&token),
        ip_address: headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned),
        user_agent: headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned),
        expires_at: now + Duration::seconds(SESSION_DURATION_SECS),
        created_at: now,
        updated_at: now,
    };
    let mut conn = crate::db::conn().await?;
    let model = diesel::insert_into(sessions::table)
        .values(&new)
        .get_result::<Session>(&mut conn)
        .await?;
    Ok(CreatedSession { model, token })
}

pub(in crate::api) fn session_model_to_view(session: &Session) -> SessionView {
    SessionView {
        id: session.id.to_string(),
        user_id: session.user_id.to_string(),
        expires_at: session.expires_at.to_rfc3339(),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        ip_address: session.ip_address.clone(),
        user_agent: session.user_agent.clone(),
    }
}

pub(in crate::api) fn user_model_to_view(user: &User) -> UserView {
    UserView {
        id: user.pid.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified_at.is_some(),
    }
}

fn unauthorized_response(headers: &HeaderMap) -> axum::response::Response {
    error_response(
        axum::http::StatusCode::UNAUTHORIZED,
        headers,
        ErrorSpec {
            error: "Authentication required".to_string(),
            code: "UNAUTHORIZED",
            details: None,
        },
    )
}
