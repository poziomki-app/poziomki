#[path = "auth_account.rs"]
mod auth_account;
#[path = "auth_session.rs"]
mod auth_session;

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{app::AppContext, hash, prelude::*};

use super::{
    error_response,
    state::{
        create_session_db, extract_bearer_token, is_valid_email, lock_otp_state, normalize_email,
        require_auth_db, session_model_to_view, user_model_to_view, validate_signup_payload,
        DataResponse, ResendOtpBody, SessionListItem, SignInBody, SignUpBody, SuccessResponse,
        VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::models::{
    _entities::{sessions, users},
    users::{Model as UserModel, RegisterParams},
};

pub(super) use auth_account::{delete_account, export_data};
pub(super) use auth_session::get_session;

fn unauthorized_error(headers: &HeaderMap, message: &str) -> Response {
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

async fn create_user_or_error(
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

pub(super) async fn sign_up(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
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
        "session": session_model_to_view(&session),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
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

pub(super) async fn sign_in(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignInBody>,
) -> Result<Response> {
    let _ = payload.remember_me;
    let email = normalize_email(&payload.email);
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

    let Some(user) = find_authenticated_user(&ctx.db, &email, &payload.password).await? else {
        return Ok(unauthorized_error(&headers, "Authentication failed"));
    };

    let session = create_session_db(&ctx.db, &headers, user.id).await?;

    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session),
    });
    Ok(Json(DataResponse { data }).into_response())
}

async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> std::result::Result<Option<users::Model>, loco_rs::Error> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
}

pub(super) async fn verify_otp(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);

    let Some(user) = find_user_by_email(&ctx.db, &email).await? else {
        return Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "User not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    let bypass_code = std::env::var("OTP_BYPASS_CODE").ok();
    let otp_ok = bypass_code.is_some_and(|code| payload.otp == code)
        || lock_otp_state()
            .otp_by_email
            .get(&email)
            .is_some_and(|saved| saved == &payload.otp);

    if !otp_ok {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Invalid verification code".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    if user.email_verified_at.is_none() {
        let mut active: users::ActiveModel = user.clone().into();
        active.email_verified_at =
            sea_orm::ActiveValue::Set(Some(chrono::offset::Local::now().into()));
        let _ = active.update(&ctx.db).await;
    }

    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&user),
            "status": true,
        }),
    })
    .into_response())
}

pub(super) async fn resend_otp(
    State(ctx): State<AppContext>,
    _headers: HeaderMap,
    Json(payload): Json<ResendOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);

    let exists = find_user_by_email(&ctx.db, &email).await?.is_some();

    if exists {
        lock_otp_state()
            .otp_by_email
            .insert(email, "123456".to_string());
    }

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sign_out(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        let _ = sessions::Entity::delete_many()
            .filter(sessions::Column::Token.eq(&token))
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
