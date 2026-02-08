use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use uuid::Uuid;

use super::{
    error_response,
    state::{
        extract_bearer_token, is_valid_email, lock_state, make_session, normalize_email,
        require_auth, resolve_session, session_to_view, to_full_profile_response, user_to_view,
        validate_signup_payload, DataResponse, DeleteAccountBody, ResendOtpBody, SessionListItem,
        SessionResponse, SignInBody, SignUpBody, SuccessResponse, UserRecord, VerifyOtpBody,
    },
    ErrorSpec,
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

pub(super) async fn get_session(headers: HeaderMap) -> Result<Response> {
    let response = {
        let mut state = lock_state();
        extract_bearer_token(&headers)
            .and_then(|token| resolve_session(&mut state, &token, Utc::now()))
            .and_then(|session| {
                state
                    .users
                    .get(&session.user_id)
                    .cloned()
                    .map(|user| (session, user))
            })
            .map_or_else(empty_session_response, |(session, user)| {
                Json(SessionResponse {
                    session: Some(session_to_view(&session)),
                    user: Some(user_to_view(&user)),
                })
                .into_response()
            })
    };
    Ok(response)
}

pub(super) async fn sign_up(
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
    let (user, session) = {
        let mut state = lock_state();
        if state.users_by_email.contains_key(&email) {
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

        let user = UserRecord {
            id: Uuid::new_v4().to_string(),
            email: email.clone(),
            name,
            password: payload.password,
            email_verified: false,
            created_at: Utc::now(),
        };
        let session = make_session(&headers, &user.id);

        state.users_by_email.insert(email, user.id.clone());
        state.users.insert(user.id.clone(), user.clone());
        state
            .sessions_by_token
            .insert(session.token.clone(), session.clone());
        drop(state);
        (user, session)
    };

    let data = serde_json::json!({
        "user": user_to_view(&user),
        "token": session.token,
        "session": session_to_view(&session),
    });
    Ok((axum::http::StatusCode::OK, Json(DataResponse { data })).into_response())
}

pub(super) async fn sign_in(
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

    let (user, session) = {
        let mut state = lock_state();
        let authorized_user = state
            .users_by_email
            .get(&email)
            .and_then(|user_id| state.users.get(user_id))
            .filter(|user| user.password == payload.password)
            .cloned();

        let Some(user) = authorized_user else {
            return Ok(unauthorized_error(&headers, "Authentication failed"));
        };

        let session = make_session(&headers, &user.id);
        state
            .sessions_by_token
            .insert(session.token.clone(), session.clone());
        drop(state);
        (user, session)
    };

    let data = serde_json::json!({
        "user": user_to_view(&user),
        "token": session.token,
        "session": session_to_view(&session),
    });
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn verify_otp(
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    let user = {
        let mut state = lock_state();
        let Some(user_id) = state.users_by_email.get(&email).cloned() else {
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
            || state
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

        if let Some(user) = state.users.get_mut(&user_id) {
            user.email_verified = true;
        }
        state.users.get(&user_id).map(user_to_view)
    };
    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user,
            "status": true,
        }),
    })
    .into_response())
}

pub(super) async fn resend_otp(
    _headers: HeaderMap,
    Json(payload): Json<ResendOtpBody>,
) -> Result<Response> {
    let email = normalize_email(&payload.email);
    {
        let mut state = lock_state();
        if state.users_by_email.contains_key(&email) {
            state.otp_by_email.insert(email, "123456".to_string());
        }
    }
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sign_out(headers: HeaderMap) -> Result<Response> {
    if let Some(token) = extract_bearer_token(&headers) {
        lock_state().sessions_by_token.remove(&token);
    }
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn sessions(headers: HeaderMap) -> Result<Response> {
    let data = {
        let mut state = lock_state();
        let (_session, user) = match require_auth(&headers, &mut state) {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };

        let now = Utc::now();
        state
            .sessions_by_token
            .values_mut()
            .filter(|session| session.user_id == user.id && session.expires_at > now)
            .map(|session| {
                if (now - session.updated_at) >= Duration::hours(24) {
                    session.updated_at = now;
                    session.expires_at = now + Duration::days(7);
                }
                SessionListItem {
                    id: session.id.clone(),
                    user_id: session.user_id.clone(),
                    expires_at: session.expires_at.to_rfc3339(),
                    created_at: session.created_at.to_rfc3339(),
                    ip_address: session.ip_address.clone(),
                    user_agent: session.user_agent.clone(),
                }
            })
            .collect::<Vec<_>>()
    };

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn delete_account(
    headers: HeaderMap,
    Json(payload): Json<DeleteAccountBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let (_session, user) = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    if payload.password.is_empty() || payload.password != user.password {
        return Ok(unauthorized_error(&headers, "Invalid password"));
    }

    state.users.remove(&user.id);
    state.users_by_email.remove(&user.email);

    if let Some(profile_id) = state.profiles_by_user.remove(&user.id) {
        state.profiles.remove(&profile_id);
    }

    state
        .sessions_by_token
        .retain(|_, existing| existing.user_id != user.id);
    drop(state);

    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn export_data(headers: HeaderMap) -> Result<Response> {
    let export = {
        let mut state = lock_state();
        let (_session, user) = match require_auth(&headers, &mut state) {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };

        let profile = state
            .profiles_by_user
            .get(&user.id)
            .and_then(|id| state.profiles.get(id))
            .cloned();
        let profile_response = profile
            .as_ref()
            .map(|profile| to_full_profile_response(&state, profile));
        drop(state);

        serde_json::json!({
            "user": {
                "id": user.id,
                "email": user.email,
                "name": user.name,
                "emailVerified": user.email_verified,
                "createdAt": user.created_at.to_rfc3339(),
            },
            "profile": profile_response,
            "tags": [],
            "events": [],
            "eventsAttended": [],
            "conversations": [],
            "messages": [],
            "sessions": [],
            "exportedAt": Utc::now().to_rfc3339(),
        })
    };
    Ok(Json(DataResponse { data: export }).into_response())
}
