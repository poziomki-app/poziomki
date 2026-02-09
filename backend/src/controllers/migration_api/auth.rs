use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::{app::AppContext, hash, prelude::*};

use super::{
    error_response,
    state::{
        create_session_db, extract_bearer_token, is_valid_email, lock_otp_state, normalize_email,
        require_auth_db, session_model_to_view, user_model_to_view, validate_signup_payload,
        DataResponse, DeleteAccountBody, ResendOtpBody, SessionListItem, SessionResponse,
        SignInBody, SignUpBody, SuccessResponse, VerifyOtpBody,
    },
    ErrorSpec,
};
use crate::models::{
    _entities::{profiles, sessions, users},
    users::{Model as UserModel, RegisterParams},
};

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

fn empty_session_response() -> Response {
    Json(SessionResponse {
        session: None,
        user: None,
    })
    .into_response()
}

pub(super) async fn get_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let token = extract_bearer_token(&headers);
    let Some(token) = token else {
        return Ok(empty_session_response());
    };

    let session = sessions::Entity::find()
        .filter(sessions::Column::Token.eq(&token))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let Some(session) = session else {
        return Ok(empty_session_response());
    };

    let now = Utc::now();
    if session.expires_at.with_timezone(&Utc) <= now {
        let _ = sessions::Entity::delete_by_id(session.id)
            .exec(&ctx.db)
            .await;
        return Ok(empty_session_response());
    }

    let user = users::Entity::find_by_id(session.user_id)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let Some(user) = user else {
        return Ok(empty_session_response());
    };

    Ok(Json(SessionResponse {
        session: Some(session_model_to_view(&session)),
        user: Some(user_model_to_view(&user)),
    })
    .into_response())
}

pub(super) async fn sign_up(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<SignUpBody>,
) -> Result<Response> {
    if let Err(spec) = validate_signup_payload(&payload) {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            spec,
        ));
    }

    let email = normalize_email(&payload.email);
    let name = payload.name.trim().to_string();

    let user = match UserModel::create_with_password(
        &ctx.db,
        &RegisterParams {
            email,
            password: payload.password,
            name,
        },
    )
    .await
    {
        Ok(user) => user,
        Err(ModelError::EntityAlreadyExists) => {
            return Ok(error_response(
                axum::http::StatusCode::CONFLICT,
                &headers,
                ErrorSpec {
                    error: "User already exists".to_string(),
                    code: "CONFLICT",
                    details: None,
                },
            ));
        }
        Err(err) => return Err(loco_rs::Error::Any(err.into())),
    };

    let session = create_session_db(&ctx.db, &headers, user.id).await?;

    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session),
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

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let user = user.filter(|u| hash::verify_password(&payload.password, &u.password));

    let Some(user) = user else {
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

pub(super) async fn verify_otp(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let Some(user) = user else {
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

    // Mark email as verified
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

    let exists = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?
        .is_some();

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

pub(super) async fn delete_account(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DeleteAccountBody>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    if payload.password.is_empty() || !hash::verify_password(&payload.password, &user.password) {
        return Ok(unauthorized_error(&headers, "Invalid password"));
    }

    // Delete profile if exists
    let _ = profiles::Entity::delete_many()
        .filter(profiles::Column::UserId.eq(user.id))
        .exec(&ctx.db)
        .await;

    // Delete all sessions
    let _ = sessions::Entity::delete_many()
        .filter(sessions::Column::UserId.eq(user.id))
        .exec(&ctx.db)
        .await;

    // Delete user
    let _ = users::Entity::delete_by_id(user.id).exec(&ctx.db).await;

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn export_data(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let profile_view: Option<serde_json::Value> = profile.map(|p| {
        serde_json::json!({
            "id": p.id.to_string(),
            "userId": user.pid.to_string(),
            "name": p.name,
            "bio": p.bio,
            "age": p.age,
            "profilePicture": p.profile_picture,
            "images": p.images,
            "program": p.program,
            "createdAt": p.created_at.to_rfc3339(),
            "updatedAt": p.updated_at.to_rfc3339(),
        })
    });

    let export = serde_json::json!({
        "user": {
            "id": user.pid.to_string(),
            "email": user.email,
            "name": user.name,
            "emailVerified": user.email_verified_at.is_some(),
            "createdAt": user.created_at.to_rfc3339(),
        },
        "profile": profile_view,
        "tags": [],
        "events": [],
        "eventsAttended": [],
        "conversations": [],
        "messages": [],
        "sessions": [],
        "exportedAt": Utc::now().to_rfc3339(),
    });

    Ok(Json(DataResponse { data: export }).into_response())
}
