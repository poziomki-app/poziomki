#[path = "otp_email.rs"]
mod auth_otp_email;
pub(super) use auth_otp_email::send_otp_email;

use axum::response::Response;
use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use diesel_async::AsyncConnection;

use super::super::{
    error_response,
    state::{
        create_session_db, normalize_email, session_model_to_view, upsert_otp, user_model_to_view,
        validate_signup_payload, verify_otp_db, DataResponse, SignUpBody,
    },
    ErrorSpec,
};
use crate::db::models::users::{NewUser, User, UserChangeset};
use crate::db::schema::{sessions, users};
use crate::jobs::enqueue_otp_email;

pub(super) const OTP_RESEND_COOLDOWN_SECS: i64 = 30;

pub(super) fn unauthorized_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::UNAUTHORIZED,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "UNAUTHORIZED",
            details: None,
        },
    )
}

pub(super) fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(super) fn generate_otp_code() -> String {
    let value = (uuid::Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{value:06}")
}

pub(super) fn invalid_otp_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: "Invalid verification code".to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

fn registration_failed_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: "Registration failed".to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    )
}

pub(super) async fn create_user_or_error(
    headers: &HeaderMap,
    payload: &SignUpBody,
) -> std::result::Result<User, Response> {
    if let Err(spec) = validate_signup_payload(payload) {
        return Err(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            spec,
        ));
    }

    let email = normalize_email(&payload.email);
    let name = payload.name.trim().to_string();

    let mut conn = crate::db::conn().await.map_err(|e| {
        tracing::error!("Pool error: {e}");
        registration_failed_response(headers)
    })?;

    let existing = users::table
        .filter(users::email.eq(&email))
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| {
            tracing::error!("User lookup failed: {e}");
            registration_failed_response(headers)
        })?;

    if let Some(user) = existing {
        if user.email_verified_at.is_some() {
            // Already verified — they should log in instead.
            return Err(error_response(
                axum::http::StatusCode::CONFLICT,
                headers,
                ErrorSpec {
                    error: "Account already exists".to_string(),
                    code: "CONFLICT",
                    details: None,
                },
            ));
        }

        // Unverified — let them proceed through OTP verification again
        // without updating credentials (prevents password-overwrite attacks).
        return Ok(user);
    }

    let password_hash = crate::security::hash_password(&payload.password).map_err(|e| {
        tracing::error!("Password hashing failed: {e}");
        registration_failed_response(headers)
    })?;

    let new_user = NewUser {
        pid: uuid::Uuid::new_v4(),
        email,
        password: password_hash,
        api_key: format!("lo-{}", uuid::Uuid::new_v4()),
        name,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!("User creation failed: {e}");
            registration_failed_response(headers)
        })
}

async fn find_authenticated_user(
    email: &str,
    password: &str,
) -> std::result::Result<Option<User>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let user = users::table
        .filter(users::email.eq(email))
        .first::<User>(&mut conn)
        .await
        .optional()?;

    Ok(user.filter(|u| crate::security::verify_password(password, &u.password)))
}

pub(super) async fn sign_in_success_or_unauthorized(
    headers: &HeaderMap,
    email: &str,
    password: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    let Some(user) = find_authenticated_user(email, password).await? else {
        return Ok(unauthorized_error(headers, "Authentication failed"));
    };

    if user.email_verified_at.is_none() {
        let code = generate_otp_code();
        upsert_otp(email, &code).await?;
        if let Err(error) = enqueue_otp_email(email, &code).await {
            tracing::error!(%error, email = %email, "failed to enqueue OTP email for unverified sign in");
        }

        return Ok(error_response(
            axum::http::StatusCode::FORBIDDEN,
            headers,
            ErrorSpec {
                error: "Email not verified".to_string(),
                code: "EMAIL_NOT_VERIFIED",
                details: None,
            },
        ));
    }

    let session = create_session_db(headers, user.id).await?;
    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session.model),
    });
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn find_user_by_email(
    email: &str,
) -> std::result::Result<Option<User>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    Ok(users::table
        .filter(users::email.eq(email))
        .first::<User>(&mut conn)
        .await
        .optional()?)
}

pub(super) async fn verify_otp_inner(
    headers: &HeaderMap,
    email: &str,
    otp: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    let Some(user) = find_user_by_email(email).await? else {
        return Ok(invalid_otp_response(headers));
    };

    let otp_ok = verify_otp_db(email, otp).await;
    if !otp_ok {
        return Ok(invalid_otp_response(headers));
    }

    let mut verified_user = user.clone();
    if verified_user.email_verified_at.is_none() {
        let now = Utc::now();
        let mut conn = crate::db::conn().await?;
        let _ = diesel::update(users::table.find(user.id))
            .set(&UserChangeset {
                email_verified_at: Some(Some(now)),
                updated_at: Some(now),
            })
            .execute(&mut conn)
            .await;
        verified_user.email_verified_at = Some(now);
    }

    let session = create_session_db(headers, verified_user.id).await?;
    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&verified_user),
            "token": session.token,
            "session": session_model_to_view(&session.model),
            "status": true,
        }),
    })
    .into_response())
}

// --- Forgot password ---

const RESET_TOKEN_TTL_SECS: i64 = 600;

fn hash_reset_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let digest = Sha256::digest(token.as_bytes());
    let mut hex = String::with_capacity(3 + 64);
    hex.push_str("rt:");
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

pub(super) async fn forgot_password_verify_inner(
    headers: &HeaderMap,
    email: &str,
    otp: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    let Some(user) = find_user_by_email(email).await? else {
        return Ok(invalid_otp_response(headers));
    };

    if user.email_verified_at.is_none() {
        return Ok(invalid_otp_response(headers));
    }

    let otp_ok = verify_otp_db(email, otp).await;
    if !otp_ok {
        return Ok(invalid_otp_response(headers));
    }

    let raw_token = uuid::Uuid::new_v4().simple().to_string();
    let hashed = hash_reset_token(&raw_token);
    let now = Utc::now();

    let mut conn = crate::db::conn().await?;
    diesel::update(users::table.find(user.id))
        .set((
            users::reset_token.eq(&hashed),
            users::reset_sent_at.eq(now),
            users::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;

    Ok(Json(DataResponse {
        data: serde_json::json!({ "resetToken": raw_token }),
    })
    .into_response())
}

pub(super) async fn reset_password_inner(
    headers: &HeaderMap,
    reset_token: &str,
    new_password: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    if !(8..=128).contains(&new_password.len()) {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Password must be between 8 and 128 characters".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let hashed_token = hash_reset_token(reset_token);
    let cutoff = Utc::now() - chrono::Duration::seconds(RESET_TOKEN_TTL_SECS);

    let mut conn = crate::db::conn().await?;
    let user = users::table
        .filter(users::reset_token.eq(&hashed_token))
        .filter(users::reset_sent_at.gt(cutoff))
        .first::<User>(&mut conn)
        .await
        .optional()?;

    let Some(user) = user else {
        return Ok(unauthorized_error(
            headers,
            "Invalid or expired reset token",
        ));
    };

    let new_hash = crate::security::hash_password(new_password)?;

    conn.transaction(|conn| {
        let new_hash = new_hash.clone();
        Box::pin(async move {
            diesel::update(users::table.find(user.id))
                .set((
                    users::password.eq(new_hash),
                    users::reset_token.eq(None::<String>),
                    users::reset_sent_at.eq(None::<chrono::DateTime<Utc>>),
                    users::updated_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;
            diesel::delete(sessions::table.filter(sessions::user_id.eq(user.id)))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        })
    })
    .await?;

    super::super::state::invalidate_auth_cache_for_user_id(user.id).await;

    let session = create_session_db(headers, user.id).await?;
    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&user),
            "token": session.token,
            "session": session_model_to_view(&session.model),
        }),
    })
    .into_response())
}
