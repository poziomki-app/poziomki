use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use loco_rs::prelude::Response;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use super::{error_response, ErrorSpec};

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

#[derive(Clone, Debug)]
struct RateLimitEntry {
    window_start: chrono::DateTime<Utc>,
    attempts: u32,
}

static AUTH_RATE_LIMITS: LazyLock<Mutex<HashMap<String, RateLimitEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

pub(super) fn enforce_rate_limit(
    headers: &HeaderMap,
    action: AuthRateLimitAction,
    subject: &str,
) -> std::result::Result<(), Box<Response>> {
    let now = Utc::now();
    let ip = client_ip(headers);
    let key = format!("{}:{subject}:{ip}", action.key_prefix());
    let mut state = AUTH_RATE_LIMITS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Periodic cleanup: always evict stale entries, cap at 1,000
    if state.len() > 1_000 {
        state.retain(|_, entry| {
            now.signed_duration_since(entry.window_start)
                < Duration::seconds(AUTH_RATE_LIMIT_WINDOW_SECS * 2)
        });
    }

    let entry = state.entry(key).or_insert(RateLimitEntry {
        window_start: now,
        attempts: 0,
    });

    if now.signed_duration_since(entry.window_start)
        >= Duration::seconds(AUTH_RATE_LIMIT_WINDOW_SECS)
    {
        entry.window_start = now;
        entry.attempts = 0;
    }

    entry.attempts = entry.attempts.saturating_add(1);
    let limited = entry.attempts > action.max_attempts();
    drop(state);

    if limited {
        Err(Box::new(rate_limit_response(headers)))
    } else {
        Ok(())
    }
}
