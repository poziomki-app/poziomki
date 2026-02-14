use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{hash, prelude::*};

use super::super::{
    error_response,
    state::{
        create_session_db, lock_otp_state, normalize_email, session_model_to_view,
        user_model_to_view, validate_signup_payload, DataResponse, SignUpBody,
    },
    ErrorSpec,
};
use crate::models::{
    _entities::users,
    users::{Model as UserModel, RegisterParams},
};

pub(super) const OTP_TTL_SECS: i64 = 60 * 10;
pub(super) const OTP_MAX_ATTEMPTS: u8 = 5;
pub(super) const OTP_RESEND_COOLDOWN_SECS: i64 = 30;
const OTP_LENGTH: usize = 6;

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

fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn otp_bypass_enabled() -> bool {
    let is_production = std::env::var("NODE_ENV")
        .ok()
        .is_some_and(|value| value.eq_ignore_ascii_case("production"));
    env_truthy("OTP_BYPASS_ENABLED") && !is_production
}

pub(super) fn generate_otp_code() -> String {
    let value = (uuid::Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{value:0OTP_LENGTH$}")
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

fn otp_bypass_matches(otp: &str) -> bool {
    otp_bypass_enabled()
        && std::env::var("OTP_BYPASS_CODE")
            .ok()
            .is_some_and(|code| otp == code)
}

pub(super) fn verify_otp_from_state(email: &str, otp: &str, now: chrono::DateTime<Utc>) -> bool {
    let mut state = lock_otp_state();
    let mut result = false;

    if let Some(saved) = state.otp_by_email.get_mut(email) {
        if saved.expires_at <= now || saved.attempts >= OTP_MAX_ATTEMPTS {
            state.otp_by_email.remove(email);
        } else if saved.code != otp {
            saved.attempts = saved.attempts.saturating_add(1);
        } else {
            state.otp_by_email.remove(email);
            result = true;
        }
    }

    drop(state);
    result
}

pub(super) async fn create_user_or_error(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    payload: &SignUpBody,
) -> std::result::Result<users::Model, Response> {
    if let Err(spec) = validate_signup_payload(payload) {
        return Err(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            spec,
        ));
    }

    let email = normalize_email(&payload.email);
    let name = payload.name.trim().to_string();

    UserModel::create_with_password(
        db,
        &RegisterParams {
            email,
            password: payload.password.clone(),
            name,
        },
    )
    .await
    .map_err(|err| match err {
        ModelError::EntityAlreadyExists => error_response(
            axum::http::StatusCode::CONFLICT,
            headers,
            ErrorSpec {
                error: "User already exists".to_string(),
                code: "CONFLICT",
                details: None,
            },
        ),
        other => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: other.to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ),
    })
}

async fn find_authenticated_user(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> std::result::Result<Option<users::Model>, loco_rs::Error> {
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    Ok(user.filter(|u| hash::verify_password(password, &u.password)))
}

pub(super) async fn sign_in_success_or_unauthorized(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    password: &str,
) -> std::result::Result<Response, loco_rs::Error> {
    let Some(user) = find_authenticated_user(db, email, password).await? else {
        return Ok(unauthorized_error(headers, "Authentication failed"));
    };

    let session = create_session_db(db, headers, user.id).await?;
    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session.model),
    });
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> std::result::Result<Option<users::Model>, loco_rs::Error> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
}

pub(super) async fn verify_otp_inner(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    otp: &str,
) -> std::result::Result<Response, loco_rs::Error> {
    let Some(user) = find_user_by_email(db, email).await? else {
        return Ok(invalid_otp_response(headers));
    };

    let otp_ok = otp_bypass_matches(otp) || verify_otp_from_state(email, otp, Utc::now());
    if !otp_ok {
        return Ok(invalid_otp_response(headers));
    }

    if user.email_verified_at.is_none() {
        let mut active: users::ActiveModel = user.clone().into();
        active.email_verified_at =
            sea_orm::ActiveValue::Set(Some(chrono::offset::Local::now().into()));
        let _ = active.update(db).await;
    }

    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&user),
            "status": true,
        }),
    })
    .into_response())
}
