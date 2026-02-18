#[path = "state_auth.rs"]
mod state_auth;
#[path = "state_types.rs"]
mod state_types;
#[path = "state_uploads.rs"]
mod state_uploads;

use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use uuid::Uuid;

use super::ErrorSpec;
use crate::models::_entities::otp_codes;
pub(super) use state_auth::*;
pub(super) use state_types::*;
pub(super) use state_uploads::*;

pub(super) const OTP_TTL_SECS: i64 = 60 * 10;
pub(super) const OTP_MAX_ATTEMPTS: i16 = 5;

// --- OTP database operations ---

/// Insert or replace an OTP code for the given email.
pub(super) async fn upsert_otp(
    db: &DatabaseConnection,
    email: &str,
    code: &str,
) -> std::result::Result<(), loco_rs::Error> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(OTP_TTL_SECS);

    // Delete any existing OTP for this email, then insert fresh
    otp_codes::Entity::delete_many()
        .filter(otp_codes::Column::Email.eq(email))
        .exec(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let model = otp_codes::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        email: ActiveValue::Set(email.to_owned()),
        code: ActiveValue::Set(code.to_owned()),
        attempts: ActiveValue::Set(0),
        expires_at: ActiveValue::Set(expires_at.into()),
        last_sent_at: ActiveValue::Set(now.into()),
        created_at: ActiveValue::Set(now.into()),
    };
    model
        .insert(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    Ok(())
}

/// Verify an OTP code against the database. Returns true if valid.
/// On success, deletes the OTP row. On failure, increments attempts.
pub(super) async fn verify_otp_db(db: &DatabaseConnection, email: &str, otp: &str) -> bool {
    use subtle::ConstantTimeEq;

    let Ok(Some(saved)) = otp_codes::Entity::find()
        .filter(otp_codes::Column::Email.eq(email))
        .one(db)
        .await
    else {
        return false;
    };

    let now = Utc::now();

    // Expired or too many attempts — delete and reject
    if saved.expires_at.with_timezone(&Utc) <= now || saved.attempts >= OTP_MAX_ATTEMPTS {
        let _ = otp_codes::Entity::delete_by_id(saved.id).exec(db).await;
        return false;
    }

    // Constant-time comparison
    if saved.code.len() != otp.len() || !bool::from(saved.code.as_bytes().ct_eq(otp.as_bytes())) {
        // Increment attempts
        let new_attempts = saved.attempts.saturating_add(1);
        let mut active: otp_codes::ActiveModel = saved.into();
        active.attempts = ActiveValue::Set(new_attempts);
        let _ = active.update(db).await;
        return false;
    }

    // Valid — delete the OTP row
    let _ = otp_codes::Entity::delete_by_id(saved.id).exec(db).await;
    true
}

/// Check if the OTP for this email is still within the resend cooldown.
pub(super) async fn otp_in_cooldown(
    db: &DatabaseConnection,
    email: &str,
    cooldown_secs: i64,
) -> bool {
    let now = Utc::now();
    otp_codes::Entity::find()
        .filter(otp_codes::Column::Email.eq(email))
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some_and(|entry| {
            entry.last_sent_at.with_timezone(&Utc) + Duration::seconds(cooldown_secs) > now
        })
}

/// Clean up expired OTP entries (called periodically or at boot).
#[allow(dead_code)]
pub(super) async fn cleanup_expired_otps(db: &DatabaseConnection) {
    let now = Utc::now();
    let _ = otp_codes::Entity::delete_many()
        .filter(otp_codes::Column::ExpiresAt.lte(now))
        .exec(db)
        .await;
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
    if error.is_none() && domain != "*" && !email.ends_with(&format!("@{domain}")) {
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

pub(super) fn validate_profile_bio(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 5_000) {
        Err("Bio must be at most 5000 characters")
    } else {
        Ok(())
    }
}

pub(super) fn validate_profile_program(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 200) {
        Err("Program must be at most 200 characters")
    } else {
        Ok(())
    }
}
