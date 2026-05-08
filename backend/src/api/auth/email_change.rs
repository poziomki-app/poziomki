use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::api::{auth_or_respond, error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db::schema::users;
use crate::db::{self, DbViewer};

use super::super::state::{
    invalidate_auth_cache_for_user_id, is_valid_email, normalize_email, otp_in_cooldown,
    upsert_otp, verify_otp_db, DataResponse, SuccessResponse,
};
use super::auth_service::{find_user_by_email, generate_otp_code, OTP_RESEND_COOLDOWN_SECS};
use crate::jobs::enqueue_otp_email;

type Result<T> = crate::error::AppResult<T>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct EmailChangeRequestBody {
    pub(in crate::api) new_email: String,
    pub(in crate::api) current_password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct EmailChangeConfirmBody {
    pub(in crate::api) new_email: String,
    pub(in crate::api) code: String,
}

#[derive(Serialize)]
struct EmailChangeResponse {
    success: bool,
    email: String,
}

fn validation_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

fn email_taken_error(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::CONFLICT,
        headers,
        ErrorSpec {
            error: "Email is already in use".to_string(),
            code: "EMAIL_TAKEN",
            details: None,
        },
    )
}

pub(in crate::api) async fn request_email_change(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<EmailChangeRequestBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if payload.current_password.is_empty()
        || !crate::security::verify_password(&payload.current_password, &user.password)
    {
        return Ok(super::auth_service::unauthorized_error(
            &headers,
            "Invalid password",
        ));
    }

    let new_email = normalize_email(&payload.new_email);
    if new_email.is_empty() || !is_valid_email(&new_email) {
        return Ok(validation_error(&headers, "Invalid email address"));
    }
    if new_email == user.email {
        return Ok(validation_error(
            &headers,
            "New email must differ from current email",
        ));
    }
    if find_user_by_email(&new_email).await?.is_some() {
        return Ok(email_taken_error(&headers));
    }
    if otp_in_cooldown(&new_email, OTP_RESEND_COOLDOWN_SECS).await {
        return Ok(error_response(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            &headers,
            ErrorSpec {
                error: "Please wait before requesting another code".to_string(),
                code: "RATE_LIMITED",
                details: None,
            },
        ));
    }

    let code = generate_otp_code();
    upsert_otp(&new_email, &code).await?;
    if let Err(error) = enqueue_otp_email(&new_email, &code).await {
        tracing::error!(
            %error,
            email = %crate::api::redact_email(&new_email),
            "failed to enqueue OTP email for email change"
        );
    }

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn confirm_email_change(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<EmailChangeConfirmBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let new_email = normalize_email(&payload.new_email);
    if new_email.is_empty() || !is_valid_email(&new_email) {
        return Ok(validation_error(&headers, "Invalid email address"));
    }
    if new_email == user.email {
        return Ok(validation_error(
            &headers,
            "New email must differ from current email",
        ));
    }

    if !verify_otp_db(&new_email, &payload.code).await {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Invalid or expired verification code".to_string(),
                code: "INVALID_OTP",
                details: None,
            },
        ));
    }

    // Re-check inside the transaction — between request and confirm someone
    // else may have signed up with this address.
    if find_user_by_email(&new_email).await?.is_some() {
        return Ok(email_taken_error(&headers));
    }

    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let new_email_for_tx = new_email.clone();
    db::with_viewer_tx(viewer, |conn| {
        async move {
            let now = Utc::now();
            diesel::update(users::table.find(user.id))
                .set((
                    users::email.eq(new_email_for_tx),
                    users::email_verified_at.eq(Some(now)),
                    users::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    invalidate_auth_cache_for_user_id(user.id).await;

    Ok(Json(DataResponse {
        data: EmailChangeResponse {
            success: true,
            email: new_email,
        },
    })
    .into_response())
}
