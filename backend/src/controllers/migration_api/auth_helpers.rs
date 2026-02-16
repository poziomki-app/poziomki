use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use loco_rs::{hash, prelude::*};
use subtle::ConstantTimeEq;

use super::super::{
    error_response,
    state::{
        create_session_db, lock_otp_state, normalize_email, session_model_to_view,
        user_model_to_view, validate_signup_payload, DataResponse, SignUpBody,
    },
    ErrorSpec,
};
use crate::models::{
    _entities::users,
    users::{Model as UserModel, RegisterParams},
};

pub(super) const OTP_TTL_SECS: i64 = 60 * 10;
pub(super) const OTP_MAX_ATTEMPTS: u8 = 5;
pub(super) const OTP_RESEND_COOLDOWN_SECS: i64 = 30;
const OTP_LENGTH: usize = 6;

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

fn env_truthy(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn otp_bypass_enabled() -> bool {
    env_truthy("OTP_BYPASS_ENABLED")
}

pub(super) fn generate_otp_code() -> String {
    let value = (uuid::Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{value:0OTP_LENGTH$}")
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

fn otp_bypass_matches(otp: &str) -> bool {
    otp_bypass_enabled()
        && std::env::var("OTP_BYPASS_CODE").ok().is_some_and(|code| {
            code.len() == otp.len() && bool::from(otp.as_bytes().ct_eq(code.as_bytes()))
        })
}

pub(super) fn verify_otp_from_state(email: &str, otp: &str, now: chrono::DateTime<Utc>) -> bool {
    let mut state = lock_otp_state();
    let mut result = false;

    if let Some(saved) = state.otp_by_email.get_mut(email) {
        if saved.expires_at <= now || saved.attempts >= OTP_MAX_ATTEMPTS {
            state.otp_by_email.remove(email);
        } else if saved.code.len() != otp.len()
            || !bool::from(saved.code.as_bytes().ct_eq(otp.as_bytes()))
        {
            saved.attempts = saved.attempts.saturating_add(1);
        } else {
            state.otp_by_email.remove(email);
            result = true;
        }
    }

    drop(state);
    result
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
        let now = Utc::now();
        let code = generate_otp_code();
        let code_for_email = code.clone();
        let email_for_send = email.to_owned();
        {
            let mut state = lock_otp_state();
            state.cleanup();
            state.otp_by_email.insert(
                email.to_owned(),
                super::super::state::OtpEntry {
                    code,
                    expires_at: now + chrono::Duration::seconds(OTP_TTL_SECS),
                    attempts: 0,
                    last_sent_at: now,
                },
            );
        }
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
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Kod weryfikacyjny Poziomki</title>
</head>
<body style="margin:0;padding:0;background:#f4f4f5;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif">
<table width="100%" cellpadding="0" cellspacing="0" style="background:#f4f4f5;padding:40px 16px">
<tr><td align="center">
<table width="100%" cellpadding="0" cellspacing="0" style="max-width:460px;background:#ffffff;border-radius:12px;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,0.08)">

<tr><td style="padding:32px 32px 0;text-align:center">
  <img src="https://mobile.poziomki.app/download/poziomki-logo.png" alt="Poziomki" width="40" height="40" style="display:inline-block;vertical-align:middle;margin-right:8px">
  <span style="font-size:20px;font-weight:700;color:#0d1117;vertical-align:middle;letter-spacing:-0.3px">Poziomki</span>
</td></tr>

<tr><td style="padding:28px 32px 12px;text-align:center">
  <p style="margin:0 0 20px;font-size:15px;color:#374151;line-height:1.5">Twój kod weryfikacyjny do aplikacji Poziomki:</p>
  <div style="margin:0 0 20px;padding:16px 24px;background:#f8fafc;border:1px solid #e2e8f0;border-radius:8px;display:inline-block">
    <span style="font-size:32px;font-weight:700;color:#0d1117;letter-spacing:6px;font-variant-numeric:tabular-nums">{code}</span>
  </div>
  <p style="margin:0;font-size:13px;color:#6b7280;line-height:1.5">Wpisz ten kod w aplikacji, aby potwierdzić swoje konto.<br>Kod wygasa za 10 minut.</p>
</td></tr>

<tr><td style="padding:20px 32px 28px">
</td></tr>

</table>
</td></tr>
</table>
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
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@mail.poziomki.app".into());

    let Ok(from_addr) = from.parse() else {
        tracing::error!("Invalid SMTP_FROM address: {from}");
        return;
    };
    let Ok(to_addr) = to.parse() else {
        tracing::error!("Invalid recipient address: {to}");
        return;
    };

    let html_body = otp_email_html(code);
    let plain_body = format!(
        "Twój kod weryfikacyjny do aplikacji Poziomki: {code}\n\nWpisz ten kod w aplikacji, aby potwierdzić swoje konto.\nKod wygasa za 10 minut."
    );

    let email = match Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(format!("{code} \u{2014} Twój kod weryfikacyjny Poziomki"))
        .multipart(
            lettre::message::MultiPart::alternative()
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(plain_body),
                )
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body),
                ),
        ) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to build OTP email: {e}");
            return;
        }
    };

    let creds = Credentials::new(user, password);
    // Accept self-signed certs for internal Docker network
    let Ok(tls) = lettre::transport::smtp::client::TlsParameters::builder(host.clone())
        .dangerous_accept_invalid_certs(true)
        .dangerous_accept_invalid_hostnames(true)
        .build()
    else {
        tracing::error!("Failed to build TLS parameters");
        return;
    };

    let Ok(transport) = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host) else {
        tracing::error!("Failed to create SMTP transport for {host}");
        return;
    };
    let mailer = transport
        .port(port)
        .credentials(creds)
        .tls(lettre::transport::smtp::client::Tls::Required(tls))
        .build();

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

    let otp_ok = otp_bypass_matches(otp) || verify_otp_from_state(email, otp, Utc::now());
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
