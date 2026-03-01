#[path = "account.rs"]
mod auth_account;
#[path = "rate_limit.rs"]
mod auth_rate_limit;
#[path = "service.rs"]
mod auth_service;
#[path = "session.rs"]
mod auth_session;

use self::auth_rate_limit::{enforce_rate_limit, AuthRateLimitAction};
use self::auth_service::{
    create_user_or_error, find_user_by_email, generate_otp_code, send_otp_email,
    sign_in_success_or_unauthorized, verify_otp_inner, OTP_RESEND_COOLDOWN_SECS,
};
type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{
    error_response,
    state::{
        extract_bearer_token, hash_session_token, invalidate_auth_cache_for_token, is_valid_email,
        normalize_email, otp_in_cooldown, require_auth_db, upsert_otp, user_model_to_view,
        DataResponse, ResendOtpBody, SessionListItem, SignInBody, SignUpBody, SuccessResponse,
        VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::db::models::sessions::Session;
use crate::db::schema::sessions;
use crate::jobs::enqueue_otp_email;

pub(super) use auth_account::{delete_account, export_data};
pub(super) use auth_session::get_session;

pub(super) async fn sign_up(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
    let normalized_email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::SignUp, &normalized_email).await
    {
        return Ok(*response);
    }

    let user = match create_user_or_error(&headers, &payload).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    // Generate and send OTP for email verification
    {
        let code = generate_otp_code();
        upsert_otp(&normalized_email, &code).await?;
        if let Err(error) = enqueue_otp_email(&normalized_email, &code).await {
            tracing::error!(%error, email = %normalized_email, "failed to enqueue OTP email after sign up");
        }
    }

    let data = serde_json::json!({
        "user": user_model_to_view(&user),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
}

fn is_invalid_credentials(email: &str, password: &str) -> bool {
    email.is_empty() || password.is_empty() || !is_valid_email(email)
}

pub(super) async fn sign_in(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignInBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) = enforce_rate_limit(&headers, AuthRateLimitAction::SignIn, &email).await {
        return Ok(*response);
    }

    if is_invalid_credentials(&email, &payload.password) {
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

    sign_in_success_or_unauthorized(&headers, &email, &payload.password).await
}

pub(super) async fn verify_otp(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::VerifyOtp, &email).await
    {
        return Ok(*response);
    }

    verify_otp_inner(&headers, &email, &payload.otp).await
}

async fn maybe_resend_otp(email: &str) -> crate::error::AppResult<()> {
    let exists = find_user_by_email(email).await?.is_some();
    if !exists || otp_in_cooldown(email, OTP_RESEND_COOLDOWN_SECS).await {
        return Ok(());
    }
    let code = generate_otp_code();
    upsert_otp(email, &code).await?;
    if let Err(error) = enqueue_otp_email(email, &code).await {
        tracing::error!(%error, email = %email, "failed to enqueue OTP email after resend");
    }
    Ok(())
}

pub(super) async fn resend_otp(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ResendOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::ResendOtp, &email).await
    {
        return Ok(*response);
    }

    maybe_resend_otp(&email).await?;
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn deliver_otp_email_job(to: &str, code: &str) {
    send_otp_email(to, code).await;
}

pub(super) async fn sign_out(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        let hashed = hash_session_token(&token);
        if let Ok(mut conn) = crate::db::conn().await {
            let _ = diesel::delete(sessions::table.filter(sessions::token.eq(&hashed)))
                .execute(&mut conn)
                .await;
        }
        invalidate_auth_cache_for_token(&token).await;
    }
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sessions(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let now = Utc::now();
    let mut conn = crate::db::conn().await?;
    let user_sessions = sessions::table
        .filter(sessions::user_id.eq(user.id))
        .filter(sessions::expires_at.gt(now))
        .load::<Session>(&mut conn)
        .await?;

    let user_pid = user.pid.to_string();
    let data = user_sessions
        .iter()
        .map(|s| SessionListItem {
            id: s.id.to_string(),
            user_id: user_pid.clone(),
            expires_at: s.expires_at.to_rfc3339(),
            created_at: s.created_at.to_rfc3339(),
            ip_address: s.ip_address.clone(),
            user_agent: s.user_agent.clone(),
        })
        .collect::<Vec<_>>();

    Ok(Json(DataResponse { data }).into_response())
}
