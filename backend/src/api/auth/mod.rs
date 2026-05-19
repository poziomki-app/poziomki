#[path = "account.rs"]
mod auth_account;
#[path = "email_change.rs"]
mod auth_email_change;
#[path = "rate_limit.rs"]
mod auth_rate_limit;
#[path = "service.rs"]
mod auth_service;
#[path = "session.rs"]
mod auth_session;
#[path = "turnstile.rs"]
mod auth_turnstile;
#[path = "welcome_email.rs"]
mod auth_welcome_email;

use crate::api::auth_or_respond;
use crate::api::ip_rate_limit::{enforce_ip_rate_limit, IpRateLimitAction};

use self::auth_rate_limit::{enforce_rate_limit, AuthRateLimitAction};
use self::auth_service::{
    create_user_or_error, find_user_by_email, forgot_password_verify_inner, generate_otp_code,
    reset_password_inner, send_otp_email, sign_in_success_or_unauthorized, verify_otp_inner,
    OTP_RESEND_COOLDOWN_SECS,
};
use self::auth_turnstile::verify_turnstile;
use self::auth_welcome_email::send_welcome_email;
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
        auth_user_row_to_view, extract_bearer_token, hash_session_token,
        invalidate_auth_cache_for_token, is_valid_email, normalize_email, otp_in_cooldown,
        upsert_otp, DataResponse, ForgotPasswordBody, ForgotPasswordVerifyBody, ResendOtpBody,
        ResetPasswordBody, SessionListItem, SignInBody, SignUpBody, SuccessResponse, VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::db::models::sessions::Session;
use crate::db::schema::sessions;
use crate::jobs::enqueue_otp_email;

pub(super) use auth_account::{change_password, delete_account, export_data};
pub(super) use auth_email_change::{confirm_email_change, request_email_change};
pub(super) use auth_session::get_session;

/// Source-tag value the landing-page pre-launch form sends. Used to
/// branch on the IP-rate-limit + Turnstile + `pre_launch_signed_up_at`
/// behaviour. Mobile clients send `source = None`.
const EARLY_ACCESS_SOURCE: &str = "landing_early_access";

pub(super) async fn sign_up(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
    let normalized_email = normalize_email(&payload.email);
    let is_early_access = payload.source.as_deref() == Some(EARLY_ACCESS_SOURCE);

    // Per-email throttle covers both flows. The IP-keyed throttle below
    // is the additional defence specifically for the public landing.
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::SignUp, &normalized_email).await
    {
        return Ok(*response);
    }

    if is_early_access {
        if let Err(response) =
            enforce_ip_rate_limit(&headers, IpRateLimitAction::EarlyAccessSignUp).await
        {
            return Ok(*response);
        }

        let token = payload.turnstile_token.as_deref().unwrap_or("");
        if let Err(reason) = verify_turnstile(token, None).await {
            tracing::warn!(reason = %reason, "early-access turnstile verification failed");
            return Ok(error_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                &headers,
                ErrorSpec {
                    error: "Captcha verification failed".to_string(),
                    code: "CAPTCHA_FAILED",
                    details: None,
                },
            ));
        }

        if !is_valid_platform_pref(payload.platform_pref.as_deref()) {
            return Ok(error_response(
                axum::http::StatusCode::BAD_REQUEST,
                &headers,
                ErrorSpec {
                    error: "Invalid platform preference".to_string(),
                    code: "VALIDATION_ERROR",
                    details: None,
                },
            ));
        }
    }

    let user = match create_user_or_error(&headers, &payload).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    if is_early_access {
        // Best-effort stamp. Logged-and-ignored on failure so a stuck
        // metadata write doesn't block the OTP-send path — the user can
        // still complete onboarding; the next admin sync can backfill.
        if let Err(error) = crate::db::set_pre_launch_signup_metadata(
            user.pid,
            payload.platform_pref.as_deref(),
            Some(EARLY_ACCESS_SOURCE),
        )
        .await
        {
            tracing::error!(%error, "failed to stamp pre-launch signup metadata");
        }
    }

    // Generate and send OTP for email verification
    {
        let code = generate_otp_code();
        upsert_otp(&normalized_email, &code).await?;
        if let Err(error) = enqueue_otp_email(&normalized_email, &code).await {
            tracing::error!(%error, email = %crate::api::redact_email(&normalized_email), "failed to enqueue OTP email after sign up");
        }
    }

    let data = serde_json::json!({
        "user": auth_user_row_to_view(&user),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
}

fn is_valid_platform_pref(value: Option<&str>) -> bool {
    matches!(value, None | Some("android" | "ios" | "either"))
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
        tracing::error!(%error, email = %crate::api::redact_email(email), "failed to enqueue OTP email after resend");
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

pub(super) async fn forgot_password(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ForgotPasswordBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::ForgotPassword, &email).await
    {
        return Ok(*response);
    }

    // Always return success to prevent email enumeration. The dispatch below
    // is best-effort — any failure along the chain falls through to the
    // shared success response.
    try_dispatch_forgot_password_otp(&email).await;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

/// Best-effort: look up the user by email, verify they're eligible for an
/// OTP send, and enqueue the reset email. Each step is a guard; silently
/// stops on the first miss so the caller can return a uniform success
/// response regardless of outcome (prevents email enumeration).
async fn try_dispatch_forgot_password_otp(email: &str) {
    let Ok(Some(user)) = find_user_by_email(email).await else {
        return;
    };
    if user.email_verified_at.is_none() {
        return;
    }
    if otp_in_cooldown(email, OTP_RESEND_COOLDOWN_SECS).await {
        return;
    }

    let code = generate_otp_code();
    if upsert_otp(email, &code).await.is_err() {
        return;
    }

    if let Err(error) = enqueue_otp_email(email, &code).await {
        tracing::error!(
            %error,
            email = %crate::api::redact_email(email),
            "failed to enqueue OTP for forgot password"
        );
    }
}

pub(super) async fn forgot_password_verify(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ForgotPasswordVerifyBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::ForgotPasswordVerify, &email).await
    {
        return Ok(*response);
    }

    forgot_password_verify_inner(&headers, &email, &payload.otp).await
}

async fn maybe_resend_forgot_password_otp(email: &str) -> crate::error::AppResult<()> {
    let user = find_user_by_email(email).await?;
    let is_verified = user.as_ref().is_some_and(|u| u.email_verified_at.is_some());
    if !is_verified || otp_in_cooldown(email, OTP_RESEND_COOLDOWN_SECS).await {
        return Ok(());
    }
    let code = generate_otp_code();
    upsert_otp(email, &code).await?;
    if let Err(error) = enqueue_otp_email(email, &code).await {
        tracing::error!(%error, email = %crate::api::redact_email(email), "failed to enqueue OTP for forgot password resend");
    }
    Ok(())
}

pub(super) async fn forgot_password_resend(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ForgotPasswordBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::ForgotPasswordResend, &email).await
    {
        return Ok(*response);
    }

    maybe_resend_forgot_password_otp(&email).await?;
    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(super) async fn reset_password(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ResetPasswordBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    if let Err(response) =
        enforce_rate_limit(&headers, AuthRateLimitAction::ResetPassword, &email).await
    {
        return Ok(*response);
    }
    reset_password_inner(
        &headers,
        &email,
        &payload.reset_token,
        &payload.new_password,
    )
    .await
}

pub(super) async fn deliver_otp_email_job(to: &str, code: &str) -> std::result::Result<(), String> {
    send_otp_email(to, code).await
}

/// Worker entrypoint for the welcome-email outbox topic.
///
/// The claim step both authorises the send (must be a pre-launch user,
/// must not already have `welcome_email_sent_at`) and reserves it
/// atomically, so a retry that lands after a successful delivery
/// becomes a no-op instead of a second send.
#[allow(dead_code)] // worker integration lands in a sibling branch
pub(super) async fn deliver_welcome_email_job(user_id: i32) -> std::result::Result<(), String> {
    let claim = crate::db::claim_welcome_email_send(user_id).await?;
    let Some(claim) = claim else {
        tracing::debug!(
            user_id,
            "welcome email claim returned no row; nothing to send"
        );
        return Ok(());
    };

    match send_welcome_email(&claim.email, &claim.name).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // We've already stamped welcome_email_sent_at via the claim. A
            // delivery failure here means we won't retry the inbox send,
            // which is intentional: the welcome email is best-effort, and
            // retrying after a hard Resend failure (bad address, suppression
            // list) just produces log noise. The user still has a working
            // account.
            tracing::warn!(user_id, error = %e, "welcome email delivery failed after claim");
            Err(e)
        }
    }
}

pub(super) async fn sign_out(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        let hashed = hash_session_token(&token);
        if let Ok(mut conn) = crate::db::conn().await {
            let _ = crate::db::delete_session_by_token(&mut conn, &hashed).await;
        }
        invalidate_auth_cache_for_token(&token).await;
    }
    Ok(Json(SuccessResponse { success: true }).into_response())
}

/// Terminate every active session for the authenticated user — the
/// "sign out everywhere" button.
///
/// Used when a token is suspected of leaking, a device was lost, or
/// the user wants a clean slate. Scoped to the caller's own
/// `user_id`, not a target; the admin ban endpoint is the
/// cross-user equivalent.
///
/// Runs the DELETE inside `db::with_viewer_tx` so RLS on `sessions`
/// (policy `sessions_viewer` requires `user_id = app.current_user_id()`)
/// sees the viewer and actually purges the rows. Without the viewer
/// context the DELETE would filter to zero rows and the endpoint
/// would lie — a 200 with nothing revoked except the in-memory cache.
pub(super) async fn sign_out_all(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    use crate::db::schema::sessions;
    use diesel_async::scoped_futures::ScopedFutureExt;
    let (_session, user) = auth_or_respond!(headers);
    let viewer = crate::db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let deleted = crate::db::with_viewer_tx(viewer, move |conn| {
        async move {
            diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                .execute(conn)
                .await
        }
        .scope_boxed()
    })
    .await;
    crate::api::state::invalidate_auth_cache_for_user_id(user.id).await;
    // Drop any live WebSocket connections — otherwise "sign out
    // everywhere" leaves the chat transport happy-pathing on a
    // previously-authenticated socket until the client reconnects.
    ctx.chat_hub.disconnect_user(user.id);
    match deleted {
        Ok(_) => Ok(Json(SuccessResponse { success: true }).into_response()),
        Err(_) => Ok(Json(SuccessResponse { success: false }).into_response()),
    }
}

pub(super) async fn sessions(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = crate::db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let caller_user_id = user.id;
    let now = Utc::now();

    let user_sessions = crate::db::with_viewer_tx(viewer, |conn| {
        use diesel_async::scoped_futures::ScopedFutureExt;
        async move {
            sessions::table
                .filter(sessions::user_id.eq(caller_user_id))
                .filter(sessions::expires_at.gt(now))
                .load::<Session>(conn)
                .await
        }
        .scope_boxed()
    })
    .await?;

    let caller_pid = user.pid.to_string();
    let data = user_sessions
        .iter()
        .map(|s| SessionListItem {
            id: s.id.to_string(),
            user_id: caller_pid.clone(),
            expires_at: s.expires_at.to_rfc3339(),
            created_at: s.created_at.to_rfc3339(),
            ip_address: s.ip_address.clone(),
            user_agent: s.user_agent.clone(),
        })
        .collect::<Vec<_>>();

    Ok(Json(DataResponse { data }).into_response())
}
