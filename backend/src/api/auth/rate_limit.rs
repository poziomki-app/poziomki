use axum::http::HeaderMap;
use axum::response::Response;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::Integer;
use diesel_async::RunQueryDsl;

use super::{error_response, ErrorSpec};

const AUTH_RATE_LIMIT_WINDOW_SECS: i64 = 60;
const AUTH_SIGN_UP_MAX_ATTEMPTS: u32 = 5;
const AUTH_SIGN_IN_MAX_ATTEMPTS: u32 = 8;
const AUTH_VERIFY_OTP_MAX_ATTEMPTS: u32 = 5;
const AUTH_RESEND_OTP_MAX_ATTEMPTS: u32 = 5;
const AUTH_FORGOT_PASSWORD_MAX_ATTEMPTS: u32 = 5;
const AUTH_FORGOT_PASSWORD_VERIFY_MAX_ATTEMPTS: u32 = 5;
const AUTH_FORGOT_PASSWORD_RESEND_MAX_ATTEMPTS: u32 = 5;
const AUTH_RESET_PASSWORD_MAX_ATTEMPTS: u32 = 5;

#[derive(Clone, Copy, Debug)]
pub(super) enum AuthRateLimitAction {
    SignUp,
    SignIn,
    VerifyOtp,
    ResendOtp,
    ForgotPassword,
    ForgotPasswordVerify,
    ForgotPasswordResend,
    ResetPassword,
}

impl AuthRateLimitAction {
    const fn max_attempts(self) -> u32 {
        match self {
            Self::SignUp => AUTH_SIGN_UP_MAX_ATTEMPTS,
            Self::SignIn => AUTH_SIGN_IN_MAX_ATTEMPTS,
            Self::VerifyOtp => AUTH_VERIFY_OTP_MAX_ATTEMPTS,
            Self::ResendOtp => AUTH_RESEND_OTP_MAX_ATTEMPTS,
            Self::ForgotPassword => AUTH_FORGOT_PASSWORD_MAX_ATTEMPTS,
            Self::ForgotPasswordVerify => AUTH_FORGOT_PASSWORD_VERIFY_MAX_ATTEMPTS,
            Self::ForgotPasswordResend => AUTH_FORGOT_PASSWORD_RESEND_MAX_ATTEMPTS,
            Self::ResetPassword => AUTH_RESET_PASSWORD_MAX_ATTEMPTS,
        }
    }

    const fn key_prefix(self) -> &'static str {
        match self {
            Self::SignUp => "auth_sign_up_email",
            Self::SignIn => "auth_sign_in_email",
            Self::VerifyOtp => "auth_verify_otp_email",
            Self::ResendOtp => "auth_resend_otp_email",
            Self::ForgotPassword => "auth_forgot_password_email",
            Self::ForgotPasswordVerify => "auth_forgot_password_verify_email",
            Self::ForgotPasswordResend => "auth_forgot_password_resend_email",
            Self::ResetPassword => "auth_reset_password",
        }
    }
}

fn rate_limit_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        headers,
        ErrorSpec {
            error: "Too many requests, try again later".to_string(),
            code: "RATE_LIMITED",
            details: None,
        },
    )
}

#[derive(QueryableByName)]
struct AttemptRow {
    #[diesel(sql_type = Integer)]
    attempts: i32,
}

async fn upsert_attempt(
    key: &str,
    window_secs: i64,
) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = crate::db::conn().await?;

    let row = diesel::sql_query(
        r"
        INSERT INTO auth_rate_limits (id, rate_key, window_start, attempts, updated_at)
        VALUES (gen_random_uuid(), $1, NOW(), 1, NOW())
        ON CONFLICT (rate_key) DO UPDATE
        SET
            window_start = CASE
                WHEN auth_rate_limits.window_start <= NOW() - make_interval(secs => $2)
                    THEN NOW()
                ELSE auth_rate_limits.window_start
            END,
            attempts = CASE
                WHEN auth_rate_limits.window_start <= NOW() - make_interval(secs => $2)
                    THEN 1
                ELSE auth_rate_limits.attempts + 1
            END,
            updated_at = NOW()
        RETURNING attempts
        ",
    )
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::BigInt, _>(window_secs)
    .get_result::<AttemptRow>(&mut conn)
    .await?;

    Ok(i64::from(row.attempts))
}

pub(super) async fn enforce_rate_limit(
    headers: &HeaderMap,
    action: AuthRateLimitAction,
    subject: &str,
) -> std::result::Result<(), Box<Response>> {
    // Key by action + subject only. Client-provided forwarding headers are spoofable
    // and should not influence auth throttling decisions.
    let key = format!("{}:{subject}", action.key_prefix());

    let attempts = match upsert_attempt(&key, AUTH_RATE_LIMIT_WINDOW_SECS).await {
        Ok(value) => value,
        Err(error) => {
            // Fail open to avoid auth outage if migration hasn't been applied yet.
            tracing::warn!(%error, rate_key = %key, "auth rate limiter unavailable; allowing request");
            return Ok(());
        }
    };

    let limited = attempts > i64::from(action.max_attempts());

    if limited {
        Err(Box::new(rate_limit_response(headers)))
    } else {
        Ok(())
    }
}
