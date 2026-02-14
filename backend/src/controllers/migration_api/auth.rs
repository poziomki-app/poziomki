#[path = "auth_account.rs"]
mod auth_account;
#[path = "auth_helpers.rs"]
mod auth_helpers;
#[path = "auth_rate_limit.rs"]
mod auth_rate_limit;
#[path = "auth_session.rs"]
mod auth_session;

use self::auth_helpers::{
    create_user_or_error, find_user_by_email, generate_otp_code, sign_in_success_or_unauthorized,
    verify_otp_inner, OTP_RESEND_COOLDOWN_SECS, OTP_TTL_SECS,
};
use self::auth_rate_limit::{enforce_rate_limit, AuthRateLimitAction};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use loco_rs::{app::AppContext, prelude::*};

use super::{
    error_response,
    state::{
        create_session_db, extract_bearer_token, hash_session_token, is_valid_email,
        lock_otp_state, normalize_email, require_auth_db, session_model_to_view,
        user_model_to_view, DataResponse, ResendOtpBody, SessionListItem, SignInBody, SignUpBody,
        SuccessResponse, VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::models::_entities::sessions;

pub(super) use auth_account::{delete_account, export_data};
pub(super) use auth_session::get_session;

pub(super) async fn sign_up(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
    let normalized_email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::SignUp, &normalized_email)
    {
        return Ok(*response);
    }

    let user = match create_user_or_error(&ctx.db, &headers, &payload).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    let session = create_session_db(&ctx.db, &headers, user.id)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session.model),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
}

pub(super) async fn sign_in(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignInBody>,
) -> Result<Response> {
    let _ = payload.remember_me;
    let email = normalize_email(&payload.email);
    if let Err(response) = enforce_rate_limit(&headers, AuthRateLimitAction::SignIn, &email) {
        return Ok(*response);
    }

    if email.is_empty() || payload.password.is_empty() || !is_valid_email(&email) {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Invalid email or password".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    sign_in_success_or_unauthorized(&ctx.db, &headers, &email, &payload.password).await
}

pub(super) async fn verify_otp(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) = enforce_rate_limit(&headers, AuthRateLimitAction::VerifyOtp, &email) {
        return Ok(*response);
    }

    verify_otp_inner(&ctx.db, &headers, &email, &payload.otp).await
}

pub(super) async fn resend_otp(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ResendOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) = enforce_rate_limit(&headers, AuthRateLimitAction::ResendOtp, &email) {
        return Ok(*response);
    }

    let exists = find_user_by_email(&ctx.db, &email).await?.is_some();

    if exists {
        let now = Utc::now();
        let mut state = lock_otp_state();
        let in_cooldown = state.otp_by_email.get(&email).is_some_and(|entry| {
            entry.last_sent_at + Duration::seconds(OTP_RESEND_COOLDOWN_SECS) > now
        });
        if !in_cooldown {
            state.otp_by_email.insert(
                email,
                super::state::OtpEntry {
                    code: generate_otp_code(),
                    expires_at: now + Duration::seconds(OTP_TTL_SECS),
                    attempts: 0,
                    last_sent_at: now,
                },
            );
        }
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sign_out(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        let hashed = hash_session_token(&token);
        let _ = sessions::Entity::delete_many()
            .filter(
                sea_orm::Condition::any()
                    .add(sessions::Column::Token.eq(&hashed))
                    .add(sessions::Column::Token.eq(&token)),
            )
            .exec(&ctx.db)
            .await;
    }
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sessions(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let now = Utc::now();
    let user_sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(user.id))
        .filter(sessions::Column::ExpiresAt.gt(now))
        .all(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let data = user_sessions
        .iter()
        .map(|s| SessionListItem {
            id: s.id.to_string(),
            user_id: s.user_id.to_string(),
            expires_at: s.expires_at.to_rfc3339(),
            created_at: s.created_at.to_rfc3339(),
            ip_address: s.ip_address.clone(),
            user_agent: s.user_agent.clone(),
        })
        .collect::<Vec<_>>();

    Ok(Json(DataResponse { data }).into_response())
}
