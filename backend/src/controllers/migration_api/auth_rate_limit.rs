use axum::http::HeaderMap;
use axum::response::Response;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use super::{error_response, ErrorSpec};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};

const AUTH_RATE_LIMIT_WINDOW_SECS: i64 = 60;
const AUTH_SIGN_UP_MAX_ATTEMPTS: u32 = 12;
const AUTH_SIGN_IN_MAX_ATTEMPTS: u32 = 20;
const AUTH_VERIFY_OTP_MAX_ATTEMPTS: u32 = 25;
const AUTH_RESEND_OTP_MAX_ATTEMPTS: u32 = 12;

#[derive(Clone, Copy, Debug)]
pub(super) enum AuthRateLimitAction {
    SignUp,
    SignIn,
    VerifyOtp,
    ResendOtp,
}

impl AuthRateLimitAction {
    const fn max_attempts(self) -> u32 {
        match self {
            Self::SignUp => AUTH_SIGN_UP_MAX_ATTEMPTS,
            Self::SignIn => AUTH_SIGN_IN_MAX_ATTEMPTS,
            Self::VerifyOtp => AUTH_VERIFY_OTP_MAX_ATTEMPTS,
            Self::ResendOtp => AUTH_RESEND_OTP_MAX_ATTEMPTS,
        }
    }

    const fn key_prefix(self) -> &'static str {
        match self {
            Self::SignUp => "auth_sign_up_email",
            Self::SignIn => "auth_sign_in_email",
            Self::VerifyOtp => "auth_verify_otp_email",
            Self::ResendOtp => "auth_resend_otp_email",
        }
    }
}

fn client_ip(headers: &HeaderMap) -> String {
    // Use the *last* x-forwarded-for entry — the one appended by our trusted
    // reverse proxy (Caddy).  Earlier entries are client-controlled and spoofable.
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_string())
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

async fn upsert_attempt(
    db: &DatabaseConnection,
    key: &str,
    window_secs: i64,
) -> std::result::Result<i64, sea_orm::DbErr> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r"
        INSERT INTO auth_rate_limits (rate_key, window_start, attempts, updated_at)
        VALUES ($1, NOW(), 1, NOW())
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
        vec![key.to_string().into(), window_secs.into()],
    );

    let row = db
        .query_one(stmt)
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("auth rate limit upsert returned no row".into()))?;
    row.try_get("", "attempts")
}

pub(super) async fn enforce_rate_limit(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    action: AuthRateLimitAction,
    subject: &str,
) -> std::result::Result<(), Box<Response>> {
    let ip = client_ip(headers);
    let key = format!("{}:{subject}:{ip}", action.key_prefix());

    let attempts = match upsert_attempt(db, &key, AUTH_RATE_LIMIT_WINDOW_SECS).await {
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
