#[path = "auth_account.rs"]
mod auth_account;
#[path = "auth_helpers.rs"]
mod auth_helpers;
#[path = "auth_rate_limit.rs"]
mod auth_rate_limit;
#[path = "auth_session.rs"]
mod auth_session;

use self::auth_helpers::{
    create_user_or_error, find_user_by_email, generate_otp_code, send_otp_email,
    sign_in_success_or_unauthorized, verify_otp_inner, OTP_RESEND_COOLDOWN_SECS,
};
use self::auth_rate_limit::{enforce_rate_limit, AuthRateLimitAction};
use crate::app::AppContext;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
#[allow(unused_imports)]
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};

use super::{
    error_response,
    state::{
        extract_bearer_token, hash_session_token, is_valid_email, normalize_email, otp_in_cooldown,
        require_auth_db, upsert_otp, user_model_to_view, DataResponse, ResendOtpBody,
        SessionListItem, SignInBody, SignUpBody, SuccessResponse, VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::models::_entities::sessions;
use crate::tasks::enqueue_otp_email;

type Result<T> = crate::error::AppResult<T>;

pub(super) use auth_account::{delete_account, export_data};
pub(super) use auth_session::get_session;

pub(super) async fn sign_up(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
    let normalized_email = normalize_email(&payload.email);
    if let Err(response) = enforce_rate_limit(
        &ctx.db,
        &headers,
        AuthRateLimitAction::SignUp,
        &normalized_email,
    )
    .await
    {
        return Ok(*response);
    }

    let user = match create_user_or_error(&ctx.db, &headers, &payload).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    // Generate and send OTP for email verification
    {
        let code = generate_otp_code();
        upsert_otp(&ctx.db, &normalized_email, &code)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
        if let Err(error) = enqueue_otp_email(&ctx.db, &normalized_email, &code).await {
            tracing::error!(%error, email = %normalized_email, "failed to enqueue OTP email after sign up");
        }
    }

    let data = serde_json::json!({
        "user": user_model_to_view(&user),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
}

pub(super) async fn sign_in(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignInBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&ctx.db, &headers, AuthRateLimitAction::SignIn, &email).await
    {
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
    if let Err(response) =
        enforce_rate_limit(&ctx.db, &headers, AuthRateLimitAction::VerifyOtp, &email).await
    {
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
    if let Err(response) =
        enforce_rate_limit(&ctx.db, &headers, AuthRateLimitAction::ResendOtp, &email).await
    {
        return Ok(*response);
    }

    let exists = find_user_by_email(&ctx.db, &email).await?.is_some();

    if exists {
        let in_cooldown = otp_in_cooldown(&ctx.db, &email, OTP_RESEND_COOLDOWN_SECS).await;
        if !in_cooldown {
            let code = generate_otp_code();
            upsert_otp(&ctx.db, &email, &code)
                .await
                .map_err(|e| crate::error::AppError::Any(e.into()))?;
            if let Err(error) = enqueue_otp_email(&ctx.db, &email, &code).await {
                tracing::error!(%error, email = %email, "failed to enqueue OTP email after resend");
            }
        }
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn deliver_otp_email_job(to: &str, code: &str) {
    send_otp_email(to, code).await;
}

pub(super) async fn sign_out(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        let hashed = hash_session_token(&token);
        let _ = sessions::Entity::delete_many()
            .filter(sessions::Column::Token.eq(&hashed))
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
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

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
