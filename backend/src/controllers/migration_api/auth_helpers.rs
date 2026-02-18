use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use lettre::{
    message::{header, Mailbox, MultiPart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use loco_rs::{hash, prelude::*};

use super::super::{
    error_response,
    state::{
        create_session_db, normalize_email, session_model_to_view, upsert_otp, user_model_to_view,
        validate_signup_payload, verify_otp_db, DataResponse, SignUpBody,
    },
    ErrorSpec,
};
use crate::models::{
    _entities::users,
    users::{Model as UserModel, RegisterParams},
};

pub(super) const OTP_RESEND_COOLDOWN_SECS: i64 = 30;

pub(super) fn unauthorized_error(headers: &HeaderMap, message: &str) -> Response {
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

pub(super) fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(super) fn generate_otp_code() -> String {
    let value = (uuid::Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{value:06}")
}

pub(super) fn invalid_otp_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: "Invalid verification code".to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

pub(super) async fn create_user_or_error(
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
        other => {
            tracing::error!("User creation failed: {other}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Registration failed".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        }
    })
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

pub(super) async fn sign_in_success_or_unauthorized(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    password: &str,
) -> std::result::Result<Response, loco_rs::Error> {
    let Some(user) = find_authenticated_user(db, email, password).await? else {
        return Ok(unauthorized_error(headers, "Authentication failed"));
    };

    if user.email_verified_at.is_none() {
        // Send a fresh OTP so the user can verify
        let code = generate_otp_code();
        let code_for_email = code.clone();
        let email_for_send = email.to_owned();
        upsert_otp(db, email, &code).await?;
        tokio::spawn(async move {
            send_otp_email(&email_for_send, &code_for_email).await;
        });

        return Ok(error_response(
            axum::http::StatusCode::FORBIDDEN,
            headers,
            ErrorSpec {
                error: "Email not verified".to_string(),
                code: "EMAIL_NOT_VERIFIED",
                details: None,
            },
        ));
    }

    let session = create_session_db(db, headers, user.id).await?;
    let data = serde_json::json!({
        "user": user_model_to_view(&user),
        "token": session.token,
        "session": session_model_to_view(&session.model),
    });
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> std::result::Result<Option<users::Model>, loco_rs::Error> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
}

fn otp_email_html(code: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="pl">
<head><meta charset="utf-8"></head>
<body style="margin:0;padding:40px 20px;font-family:sans-serif;text-align:center">
<a href="https://poziomki.app"><img src="https://mobile.poziomki.app/download/poziomki-wordmark.png" alt="poziomki.app" width="280" style="display:block;margin:0 auto 32px"></a>
<p style="margin:0 0 24px;font-size:16px;color:#374151">Tw&#243;j kod logowania:</p>
<p style="margin:0 0 24px;font-size:42px;font-weight:bold;letter-spacing:10px;color:#111">{code}</p>
<p style="margin:0 0 32px;font-size:14px;color:#6b7280;line-height:1.6">Wpisz ten kod w aplikacji, aby potwierdzi&#263; swoje konto.<br>Kod wygasa za 10 minut.</p>
<a href="https://poziomki.app" style="font-size:12px;color:#9ca3af;text-decoration:none">poziomki.app</a>
</body>
</html>"#
    )
}

pub(super) async fn send_otp_email(to: &str, code: &str) {
    if !env_truthy("SMTP_ENABLE") {
        tracing::debug!("SMTP disabled, skipping OTP email to {to}");
        return;
    }

    let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into());
    let port: u16 = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(587);
    let user = std::env::var("SMTP_USER").unwrap_or_default();
    let password = std::env::var("SMTP_PASSWORD").unwrap_or_default();
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@poziomki.app".into());

    let Ok(from_addr) = from.parse() else {
        tracing::error!("Invalid SMTP_FROM address: {from}");
        return;
    };
    let from_mbox = Mailbox::new(
        Some("poziomki \u{2013} poznajmy si\u{0119}!".to_string()),
        from_addr,
    );
    let Ok(to_addr) = to.parse() else {
        tracing::error!("Invalid recipient address: {to}");
        return;
    };

    let html_body = otp_email_html(code);
    let plain_body = format!(
        "Tw\u{00f3}j kod logowania: {code}\n\nWpisz ten kod w aplikacji, aby potwierdzi\u{0107} swoje konto.\nKod wygasa za 10 minut.\n\npoziomki.app"
    );

    // Generate a proper Message-ID with our domain (not container hostname)
    let msg_id = format!("<{}@poziomki.app>", uuid::Uuid::new_v4());

    let email = match Message::builder()
        .from(from_mbox)
        .to(to_addr)
        .message_id(Some(msg_id.clone()))
        .subject(format!("Tw\u{00f3}j kod logowania to {code}"))
        .raw_header(header::HeaderValue::new(
            header::HeaderName::new_from_ascii_str("Auto-Submitted"),
            "auto-generated".to_owned(),
        ))
        .raw_header(header::HeaderValue::new(
            header::HeaderName::new_from_ascii_str("X-Auto-Response-Suppress"),
            "All".to_owned(),
        ))
        .raw_header(header::HeaderValue::new(
            header::HeaderName::new_from_ascii_str("Feedback-ID"),
            "otp:poziomki:poziomki.app".to_owned(),
        ))
        .raw_header(header::HeaderValue::new(
            header::HeaderName::new_from_ascii_str("Precedence"),
            "transactional".to_owned(),
        ))
        .multipart(MultiPart::alternative_plain_html(plain_body, html_body))
    {
        Ok(msg) => {
            tracing::info!("OTP email prepared for delivery to {to}");
            msg
        }
        Err(e) => {
            tracing::error!("Failed to build OTP email: {e}");
            return;
        }
    };

    let creds = Credentials::new(user, password);
    let Ok(transport) = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host) else {
        tracing::error!("Failed to create SMTP transport for {host}");
        return;
    };
    let mailer = transport.port(port).credentials(creds).build();

    if let Err(e) = mailer.send(email).await {
        tracing::error!("Failed to send OTP email to {to}: {e}");
    } else {
        tracing::info!("OTP email sent to {to}");
    }
}

pub(super) async fn verify_otp_inner(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    otp: &str,
) -> std::result::Result<Response, loco_rs::Error> {
    let Some(user) = find_user_by_email(db, email).await? else {
        return Ok(invalid_otp_response(headers));
    };

    let otp_ok = verify_otp_db(db, email, otp).await;
    if !otp_ok {
        return Ok(invalid_otp_response(headers));
    }

    if user.email_verified_at.is_none() {
        let mut active: users::ActiveModel = user.clone().into();
        active.email_verified_at = sea_orm::ActiveValue::Set(Some(Utc::now().into()));
        let _ = active.update(db).await;
    }

    let session = create_session_db(db, headers, user.id).await?;
    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&user),
            "token": session.token,
            "session": session_model_to_view(&session.model),
            "status": true,
        }),
    })
    .into_response())
}
