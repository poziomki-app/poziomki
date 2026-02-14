use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use uuid::Uuid;

use super::super::{error_response, ErrorSpec};
use super::{SessionView, UserView};
use crate::models::_entities::{sessions, users};

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;
const SESSION_TOKEN_HASH_PREFIX: &str = "st2:";

pub(in crate::controllers::migration_api) fn extract_bearer_token(
    headers: &HeaderMap,
) -> Option<String> {
    let header = headers.get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct CreatedSession {
    pub(in crate::controllers::migration_api) model: sessions::Model,
    pub(in crate::controllers::migration_api) token: String,
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

fn is_hashed_session_token(token: &str) -> bool {
    token.starts_with(SESSION_TOKEN_HASH_PREFIX)
}

pub(in crate::controllers::migration_api) fn hash_session_token(token: &str) -> String {
    session_token_hash(token)
}

pub(in crate::controllers::migration_api) async fn resolve_session_by_token(
    db: &DatabaseConnection,
    token: &str,
) -> std::result::Result<Option<sessions::Model>, loco_rs::Error> {
    let hashed = session_token_hash(token);
    let hashed_session = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(&hashed))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    if hashed_session.is_some() {
        return Ok(hashed_session);
    }

    let legacy = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(token))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    if let Some(model) = legacy {
        let mut active: sessions::ActiveModel = model.clone().into();
        active.token = ActiveValue::Set(hashed);
        let migrated = active
            .update(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;
        return Ok(Some(migrated));
    }

    Ok(None)
}

pub(in crate::controllers::migration_api) async fn require_auth_db(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> std::result::Result<(sessions::Model, users::Model), Box<Response>> {
    let token =
        extract_bearer_token(headers).ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let session = resolve_session_by_token(db, &token)
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let now = Utc::now();
    if session.expires_at.with_timezone(&Utc) <= now {
        let _ = sessions::Entity::delete_by_id(session.id).exec(db).await;
        return Err(Box::new(unauthorized_response(headers)));
    }

    let elapsed = now - session.updated_at.with_timezone(&Utc);
    if elapsed >= Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        let new_expires = now + Duration::seconds(SESSION_DURATION_SECS);
        let mut active: sessions::ActiveModel = session.clone().into();
        active.updated_at = ActiveValue::Set(now.into());
        active.expires_at = ActiveValue::Set(new_expires.into());
        let _ = active.update(db).await;
    }

    let user = users::Entity::find_by_id(session.user_id)
        .one(db)
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    Ok((session, user))
}

pub(in crate::controllers::migration_api) async fn create_session_db(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<CreatedSession, loco_rs::Error> {
    let now = Utc::now();
    let session_id = Uuid::new_v4();
    let secret = Uuid::new_v4().simple().to_string();
    let token = format!("{session_id}.{secret}");
    let session = sessions::ActiveModel {
        id: ActiveValue::Set(session_id),
        user_id: ActiveValue::Set(user_id),
        token: ActiveValue::Set(session_token_hash(&token)),
        ip_address: ActiveValue::Set(
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(ToOwned::to_owned),
        ),
        user_agent: ActiveValue::Set(
            headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(ToOwned::to_owned),
        ),
        expires_at: ActiveValue::Set((now + Duration::seconds(SESSION_DURATION_SECS)).into()),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };
    let model = session
        .insert(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    Ok(CreatedSession { model, token })
}

pub(in crate::controllers::migration_api) async fn migrate_legacy_session_tokens(
    db: &DatabaseConnection,
) -> std::result::Result<(), loco_rs::Error> {
    let rows = sessions::Entity::find()
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    for row in rows {
        if is_hashed_session_token(&row.token) {
            continue;
        }
        let mut active: sessions::ActiveModel = row.clone().into();
        active.token = ActiveValue::Set(session_token_hash(&row.token));
        let _ = active
            .update(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;
    }

    Ok(())
}

pub(in crate::controllers::migration_api) fn session_model_to_view(
    session: &sessions::Model,
) -> SessionView {
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

pub(in crate::controllers::migration_api) fn user_model_to_view(user: &users::Model) -> UserView {
    UserView {
        id: user.pid.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified_at.is_some(),
    }
}

fn unauthorized_response(headers: &HeaderMap) -> Response {
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
