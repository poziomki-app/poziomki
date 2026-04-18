use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::super::{error_response, ErrorSpec};
use super::{SessionView, UserView};
use crate::db::models::sessions::{Session, SessionUpdate};
use crate::db::models::users::User;
use crate::db::schema::{sessions, users};
use crate::db::{self, DbViewer};

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;
/// Hard cap from session creation: renewals cannot extend a session past this
/// absolute age, even if the user stays active. Limits how long a stolen
/// token can be kept alive by an attacker touching the API once a day.
const SESSION_MAX_ABSOLUTE_SECS: i64 = 60 * 60 * 24 * 30;

fn session_hard_expiry(created_at: DateTime<Utc>) -> DateTime<Utc> {
    created_at + Duration::seconds(SESSION_MAX_ABSOLUTE_SECS)
}

fn is_past_absolute_cap(session: &Session, now: DateTime<Utc>) -> bool {
    now >= session_hard_expiry(session.created_at)
}

fn capped_expiry(session_created_at: DateTime<Utc>, sliding: DateTime<Utc>) -> DateTime<Utc> {
    let hard = session_hard_expiry(session_created_at);
    if sliding < hard {
        sliding
    } else {
        hard
    }
}
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
    if entry.cached_until <= now
        || entry.session.expires_at <= now
        || is_past_absolute_cap(&entry.session, now)
    {
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
            maybe_renew_session(&session, user.is_review_stub).await;
        }
        tracing::Span::current().record("user_id", user.id);
        return Ok(AuthContext { session, user });
    }

    // The SECURITY DEFINER `app.resolve_session` bypasses RLS so we can
    // authenticate a bearer token before a viewer context exists. Once we
    // know the user, we set `app.user_id` / `app.is_stub` inside the same
    // transaction so the follow-up User SELECT and session renewal run under
    // the correct RLS scope.
    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?;
    let now = Utc::now();

    let hashed_for_tx = hashed.clone();
    let result = conn
        .transaction::<Option<(Session, User)>, diesel::result::Error, _>(|conn| {
            async move {
                let Some(row) = db::resolve_session(conn, &hashed_for_tx).await? else {
                    return Ok(None);
                };
                let session = session_from_row(&row);
                if session.expires_at <= now || is_past_absolute_cap(&session, now) {
                    let _ = diesel::delete(sessions::table.find(session.id))
                        .execute(conn)
                        .await;
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
                    .select(User::as_select())
                    .first::<User>(conn)
                    .await?;
                let session = maybe_renew_session_on_conn(session, conn).await;
                Ok(Some((session, user)))
            }
            .scope_boxed()
        })
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?;

    let (session, user) = result.ok_or_else(|| Box::new(unauthorized_response(headers)))?;
    cache_auth_put(hashed, &session, &user).await;
    tracing::Span::current().record("user_id", user.id);
    Ok(AuthContext { session, user })
}

fn session_from_row(row: &db::AuthSessionRow) -> Session {
    Session {
        id: row.session_id,
        user_id: row.user_id,
        token: row.token.clone(),
        ip_address: row.ip_address.clone(),
        user_agent: row.user_agent.clone(),
        expires_at: row.expires_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn maybe_renew_session(session: &Session, is_review_stub: bool) {
    let now = Utc::now();
    let elapsed = now - session.updated_at;
    if elapsed < Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        return;
    }
    let new_expires = capped_expiry(
        session.created_at,
        now + Duration::seconds(SESSION_DURATION_SECS),
    );
    let session_id = session.id;
    // Renewal is a mutation on the sessions row; run under the caller's
    // viewer context so it obeys RLS once Tier-A policies land.
    let _ = db::with_viewer_tx(
        DbViewer {
            user_id: session.user_id,
            is_review_stub,
        },
        move |conn| {
            async move {
                diesel::update(sessions::table.find(session_id))
                    .set(&SessionUpdate {
                        updated_at: Some(now),
                        expires_at: Some(new_expires),
                    })
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .scope_boxed()
        },
    )
    .await;
}

async fn maybe_renew_session_on_conn(
    session: Session,
    conn: &mut diesel_async::AsyncPgConnection,
) -> Session {
    let now = Utc::now();
    if now - session.updated_at < Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        return session;
    }

    let new_expires = capped_expiry(
        session.created_at,
        now + Duration::seconds(SESSION_DURATION_SECS),
    );
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
    let expires_at = now + Duration::seconds(SESSION_DURATION_SECS);
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);

    // The row id doubles as the first segment of the bearer token. Generate
    // both client-side, then persist via the SECURITY DEFINER helper so the
    // insert works before any viewer context is established (sign-in / OTP
    // verify / password reset all land here pre-auth).
    let session_id = Uuid::new_v4();
    let secret = Uuid::new_v4().simple().to_string();
    let token = format!("{session_id}.{secret}");
    let token_hash = session_token_hash(&token);

    let mut conn = crate::db::conn().await?;
    let row = crate::db::create_session_for_user(
        &mut conn,
        session_id,
        user_id,
        &token_hash,
        ip_address.as_deref(),
        user_agent.as_deref(),
        now,
        expires_at,
    )
    .await?;

    let model = Session {
        id: row.id,
        user_id: row.user_id,
        token: row.token,
        ip_address: row.ip_address,
        user_agent: row.user_agent,
        expires_at: row.expires_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };
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

pub(in crate::api) fn auth_user_row_to_view(row: &db::AuthUserRow) -> UserView {
    UserView {
        id: row.pid.to_string(),
        email: row.email.clone(),
        name: row.name.clone(),
        email_verified: row.email_verified_at.is_some(),
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
