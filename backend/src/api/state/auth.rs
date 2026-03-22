use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::super::{error_response, ErrorSpec};
use super::{SessionView, UserView};
use crate::db::models::sessions::{NewSession, Session, SessionUpdate};
use crate::db::models::users::User;
use crate::db::schema::{sessions, users};

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;
const AUTH_CACHE_TTL_DEFAULT_SECS: i64 = 15;
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

#[derive(Clone, Debug)]
pub(in crate::api) struct AuthContext {
    pub(in crate::api) session: Session,
    pub(in crate::api) user: User,
}

#[derive(Clone)]
struct CachedAuthEntry {
    session: Session,
    user: User,
    cached_until: DateTime<Utc>,
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

fn auth_cache() -> &'static RwLock<HashMap<String, CachedAuthEntry>> {
    static AUTH_CACHE: OnceLock<RwLock<HashMap<String, CachedAuthEntry>>> = OnceLock::new();
    AUTH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn auth_cache_ttl() -> Duration {
    static AUTH_CACHE_TTL: OnceLock<Duration> = OnceLock::new();
    *AUTH_CACHE_TTL.get_or_init(|| {
        let ttl_secs = env::var("AUTH_CACHE_TTL_SECS")
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|value| *value >= 0)
            .unwrap_or(AUTH_CACHE_TTL_DEFAULT_SECS);
        Duration::seconds(ttl_secs)
    })
}

async fn cache_auth_get(hashed_token: &str) -> Option<(Session, User)> {
    let now = Utc::now();
    let cached = {
        let guard = auth_cache().read().await;
        guard.get(hashed_token).cloned()
    };
    let entry = cached?;
    if entry.cached_until <= now || entry.session.expires_at <= now {
        auth_cache().write().await.remove(hashed_token);
        return None;
    }
    Some((entry.session, entry.user))
}

async fn cache_auth_put(hashed_token: String, session: &Session, user: &User) {
    let now = Utc::now();
    let ttl_until = now + auth_cache_ttl();
    let cached_until = if session.expires_at < ttl_until {
        session.expires_at
    } else {
        ttl_until
    };
    auth_cache().write().await.insert(
        hashed_token,
        CachedAuthEntry {
            session: session.clone(),
            user: user.clone(),
            cached_until,
        },
    );
}

pub(in crate::api) async fn invalidate_auth_cache_for_token(token: &str) {
    auth_cache()
        .write()
        .await
        .remove(&session_token_hash(token));
}

pub(in crate::api) async fn invalidate_auth_cache_for_user_id(user_id: i32) {
    auth_cache()
        .write()
        .await
        .retain(|_, entry| entry.user.id != user_id);
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
    let context = require_auth_context(headers).await?;
    Ok((context.session, context.user))
}

pub(in crate::api) async fn require_auth_context(
    headers: &HeaderMap,
) -> std::result::Result<AuthContext, Box<axum::response::Response>> {
    let token =
        extract_bearer_token(headers).ok_or_else(|| Box::new(unauthorized_response(headers)))?;
    let hashed = session_token_hash(&token);

    if let Some((session, user)) = cache_auth_get(&hashed).await {
        let elapsed = Utc::now() - session.updated_at;
        if elapsed >= Duration::seconds(SESSION_UPDATE_AGE_SECS) {
            maybe_renew_session(&session).await;
        }
        return Ok(AuthContext { session, user });
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?;
    let (session, user) = sessions::table
        .inner_join(users::table.on(users::id.eq(sessions::user_id)))
        .filter(sessions::token.eq(&hashed))
        .select((Session::as_select(), User::as_select()))
        .first::<(Session, User)>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let now = Utc::now();
    if session.expires_at <= now {
        let _ = diesel::delete(sessions::table.find(session.id))
            .execute(&mut conn)
            .await;
        return Err(Box::new(unauthorized_response(headers)));
    }

    let session = maybe_renew_session_on_conn(session, &mut conn).await;
    cache_auth_put(hashed, &session, &user).await;
    Ok(AuthContext { session, user })
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

async fn maybe_renew_session_on_conn(session: Session, conn: &mut crate::db::DbConn) -> Session {
    let now = Utc::now();
    if now - session.updated_at < Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        return session;
    }

    let new_expires = now + Duration::seconds(SESSION_DURATION_SECS);
    let _ = diesel::update(sessions::table.find(session.id))
        .set(&SessionUpdate {
            updated_at: Some(now),
            expires_at: Some(new_expires),
        })
        .execute(conn)
        .await;

    Session {
        updated_at: now,
        expires_at: new_expires,
        ..session
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
