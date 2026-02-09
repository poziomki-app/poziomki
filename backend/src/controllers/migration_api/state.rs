#[path = "state_types.rs"]
mod state_types;
#[path = "state_uploads.rs"]
mod state_uploads;

use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};
use uuid::Uuid;

use super::{error_response, ErrorSpec};
use crate::models::_entities::{sessions, users};
pub(super) use state_types::*;
pub(super) use state_uploads::*;

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;

// --- OTP in-memory state (sole remaining in-memory data) ---

#[derive(Default)]
pub(super) struct OtpState {
    pub(super) otp_by_email: HashMap<String, String>,
}

static OTP_STATE: LazyLock<Mutex<OtpState>> = LazyLock::new(|| Mutex::new(OtpState::default()));

pub(super) fn lock_otp_state() -> MutexGuard<'static, OtpState> {
    OTP_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn reset_otp_state() {
    let mut state = lock_otp_state();
    *state = OtpState::default();
}

pub(super) fn reset_state() {
    reset_otp_state();
}

// --- DB-backed auth helpers ---

pub(super) fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let header = headers.get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

pub(super) async fn require_auth_db(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> std::result::Result<(sessions::Model, users::Model), Box<Response>> {
    let token =
        extract_bearer_token(headers).ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let session = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(&token))
        .one(db)
        .await
        .map_err(|_| Box::new(unauthorized_response(headers)))?
        .ok_or_else(|| Box::new(unauthorized_response(headers)))?;

    let now = Utc::now();
    if session.expires_at.with_timezone(&Utc) <= now {
        let _ = sessions::Entity::delete_by_id(session.id).exec(db).await;
        return Err(Box::new(unauthorized_response(headers)));
    }

    // Refresh session if stale
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

pub(super) async fn create_session_db(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<sessions::Model, loco_rs::Error> {
    let now = Utc::now();
    let token = Uuid::new_v4().to_string();
    let session = sessions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user_id),
        token: ActiveValue::Set(token),
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
    session
        .insert(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
}

// --- View helpers ---

pub(super) fn session_model_to_view(session: &sessions::Model) -> SessionView {
    SessionView {
        id: session.id.to_string(),
        user_id: session.user_id.to_string(),
        token: session.token.clone(),
        expires_at: session.expires_at.to_rfc3339(),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        ip_address: session.ip_address.clone(),
        user_agent: session.user_agent.clone(),
    }
}

pub(super) fn user_model_to_view(user: &users::Model) -> UserView {
    UserView {
        id: user.pid.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified_at.is_some(),
    }
}

// --- Auth validation helpers ---

pub(super) fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub(super) fn is_valid_email(email: &str) -> bool {
    let mut split = email.split('@');
    let local = split.next();
    let domain = split.next();
    local.is_some_and(|part| !part.is_empty())
        && domain.is_some_and(|part| part.contains('.'))
        && split.next().is_none()
}

pub(super) fn allowed_email_domain() -> String {
    std::env::var("ALLOWED_EMAIL_DOMAIN").unwrap_or_else(|_| "example.com".to_string())
}

pub(super) fn validate_signup_payload(payload: &SignUpBody) -> std::result::Result<(), ErrorSpec> {
    let email = normalize_email(&payload.email);
    let mut error: Option<ErrorSpec> = None;
    if email.is_empty() {
        error = Some(validation_error_spec("Email is required"));
    } else if !is_valid_email(&email) {
        error = Some(validation_error_spec("Invalid email address"));
    } else if !(1..=100).contains(&payload.name.trim().chars().count()) {
        error = Some(validation_error_spec(
            "Name must be between 1 and 100 characters",
        ));
    } else if !(8..=128).contains(&payload.password.len()) {
        error = Some(validation_error_spec(
            "Password must be between 8 and 128 characters",
        ));
    }
    let domain = allowed_email_domain();
    if error.is_none() && !email.ends_with(&format!("@{domain}")) {
        error = Some(validation_error_spec(&format!(
            "Only @{domain} emails are allowed"
        )));
    }
    error.map_or(Ok(()), Err)
}

fn validation_error_spec(message: &str) -> ErrorSpec {
    ErrorSpec {
        error: message.to_string(),
        code: "VALIDATION_ERROR",
        details: None,
    }
}

pub(super) fn validate_profile_name(name: &str) -> std::result::Result<(), &'static str> {
    if name.trim().is_empty() {
        Err("Name is required")
    } else if name.chars().count() > 100 {
        Err("Name must be at most 100 characters")
    } else if name.contains("http://") || name.contains("https://") || name.contains("www.") {
        Err("Imie nie moze zawierac linkow ani adresow email")
    } else {
        Ok(())
    }
}

pub(super) fn validate_profile_age(age: u8) -> std::result::Result<(), &'static str> {
    if !(15..=67).contains(&age) {
        return Err("Age must be between 15 and 67");
    }
    Ok(())
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
