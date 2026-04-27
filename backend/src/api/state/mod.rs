#[path = "auth.rs"]
mod auth;
#[path = "auth_requests.rs"]
mod auth_requests;
#[path = "auth_responses.rs"]
mod auth_responses;
#[path = "catalog_requests.rs"]
mod catalog_requests;
#[path = "catalog_responses.rs"]
mod catalog_responses;
#[path = "event_requests.rs"]
mod event_requests;
#[path = "event_responses.rs"]
mod event_responses;
#[path = "matching_requests.rs"]
mod matching_requests;
#[path = "matching_responses.rs"]
mod matching_responses;
#[path = "profile_requests.rs"]
mod profile_requests;
#[path = "profile_responses.rs"]
mod profile_responses;
#[path = "response_common.rs"]
mod response_common;
#[path = "shared.rs"]
mod shared;
#[path = "uploads.rs"]
mod uploads;
#[path = "uploads_payloads.rs"]
mod uploads_payloads;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::ErrorSpec;
use crate::db;
use crate::db::models::otp_codes::{NewOtpCode, OtpCode};
use crate::db::schema::otp_codes;
pub(super) use auth::*;
pub(super) use auth_requests::*;
pub(super) use auth_responses::*;
pub(super) use catalog_requests::*;
pub(super) use catalog_responses::*;
pub(super) use event_requests::*;
pub(super) use event_responses::*;
pub(super) use matching_requests::*;
pub(super) use matching_responses::*;
pub(super) use profile_requests::*;
pub(super) use profile_responses::*;
pub(super) use response_common::*;
pub(super) use shared::*;
pub(super) use uploads::*;
pub(super) use uploads_payloads::*;

pub(super) const OTP_TTL_SECS: i64 = 60 * 10;
pub(super) const OTP_MAX_ATTEMPTS: i16 = 5;

// --- OTP database operations ---

/// Insert or replace an OTP code for the given email.
pub(super) async fn upsert_otp(
    email: &str,
    code: &str,
) -> std::result::Result<(), crate::error::AppError> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(OTP_TTL_SECS);
    let new = NewOtpCode {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        code: code.to_owned(),
        attempts: 0,
        expires_at,
        last_sent_at: now,
        created_at: now,
    };
    let email_for_delete = email.to_owned();
    db::with_anon_tx(move |conn| {
        async move {
            diesel::delete(otp_codes::table.filter(otp_codes::email.eq(email_for_delete)))
                .execute(conn)
                .await?;
            diesel::insert_into(otp_codes::table)
                .values(&new)
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;
    Ok(())
}

/// Verify an OTP code against the database. Returns true if valid.
/// On success, deletes the OTP row. On failure, increments attempts.
///
/// All failure branches perform the same constant-time comparison and an
/// equivalent DB round-trip so an attacker cannot distinguish "email has no
/// OTP row" (i.e. unknown account) from "bad code" via timing.
pub(super) async fn verify_otp_db(email: &str, otp: &str) -> bool {
    use subtle::ConstantTimeEq;

    let otp_bytes = otp.as_bytes().to_vec();
    let email = email.to_owned();

    let result: Result<bool, diesel::result::Error> = db::with_anon_tx(move |conn| {
        async move {
            let saved = otp_codes::table
                .filter(otp_codes::email.eq(&email))
                .first::<OtpCode>(conn)
                .await
                .optional()?;
            let now = Utc::now();
            let valid = saved.filter(|s| s.expires_at > now && s.attempts < OTP_MAX_ATTEMPTS);

            // Always perform the constant-time compare so CPU timing is uniform.
            let ct_match = valid.as_ref().map_or_else(
                || {
                    let dummy = vec![0u8; otp_bytes.len()];
                    let _ = bool::from(dummy.as_slice().ct_eq(&otp_bytes));
                    false
                },
                |s| {
                    s.code.len() == otp_bytes.len()
                        && bool::from(s.code.as_bytes().ct_eq(&otp_bytes))
                },
            );

            let Some(saved) = valid else {
                // No valid OTP row. Issue a no-op write to match the DB
                // round-trip of the "bad code" path, then fail.
                diesel::update(otp_codes::table.find(Uuid::nil()))
                    .set(otp_codes::attempts.eq(0))
                    .execute(conn)
                    .await?;
                return Ok(false);
            };

            if !ct_match {
                let new_attempts = saved.attempts.saturating_add(1);
                diesel::update(otp_codes::table.find(saved.id))
                    .set(otp_codes::attempts.eq(new_attempts))
                    .execute(conn)
                    .await?;
                return Ok(false);
            }

            diesel::delete(otp_codes::table.find(saved.id))
                .execute(conn)
                .await?;
            Ok(true)
        }
        .scope_boxed()
    })
    .await;

    result.unwrap_or(false)
}

/// Check if the OTP for this email is still within the resend cooldown.
pub(super) async fn otp_in_cooldown(email: &str, cooldown_secs: i64) -> bool {
    let now = Utc::now();
    let email = email.to_owned();
    let entry = db::with_anon_tx(move |conn| {
        async move {
            otp_codes::table
                .filter(otp_codes::email.eq(email))
                .first::<OtpCode>(conn)
                .await
                .optional()
        }
        .scope_boxed()
    })
    .await
    .ok()
    .flatten();
    entry.is_some_and(|e| e.last_sent_at + Duration::seconds(cooldown_secs) > now)
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

pub(super) fn validate_profile_status(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 160) {
        Err("Status must be at most 160 characters")
    } else {
        Ok(())
    }
}
