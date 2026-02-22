use axum::response::Response;
use axum::{http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use lettre::{
    message::{header, Mailbox, MultiPart},
    transport::smtp::{authentication::Credentials, client::TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};

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
    users::{Model as UserModel, ModelError, RegisterParams},
};
use crate::security;
use crate::tasks::enqueue_otp_email;
use sea_orm::DatabaseConnection;

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
) -> std::result::Result<Option<users::Model>, crate::error::AppError> {
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(user.filter(|u| security::verify_password(password, &u.password)))
}

pub(super) async fn sign_in_success_or_unauthorized(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    password: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    let Some(user) = find_authenticated_user(db, email, password).await? else {
        return Ok(unauthorized_error(headers, "Authentication failed"));
    };

    if user.email_verified_at.is_none() {
        // Send a fresh OTP so the user can verify
        let code = generate_otp_code();
        upsert_otp(db, email, &code).await?;
        if let Err(error) = enqueue_otp_email(db, email, &code).await {
            tracing::error!(%error, email = %email, "failed to enqueue OTP email for unverified sign in");
        }

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
) -> std::result::Result<Option<users::Model>, crate::error::AppError> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
}

fn otp_email_html(code: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="pl">
<head><meta charset="utf-8"></head>
<body style="margin:0;padding:40px 20px;font-family:sans-serif;text-align:center">
<p style="margin:0 0 24px;font-size:16px;color:#374151">Tw&#243;j kod logowania:</p>
<p style="margin:0 0 24px;font-size:42px;font-weight:bold;letter-spacing:10px;color:#111">{code}</p>
<p style="margin:0;font-size:14px;color:#6b7280;line-height:1.6">Wpisz ten kod w aplikacji, aby potwierdzi&#263; swoje konto.<br>Kod wygasa za 10 minut.</p>
<p style="margin:20px 0 0;font-size:12px;color:#9ca3af">2026 poziomki 🩵</p>
</body>
</html>"#
    )
}

struct SmtpSettings {
    host: String,
    port: u16,
    user: String,
    password: String,
    from: String,
    tls_name: String,
}

fn smtp_settings() -> SmtpSettings {
    let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(587);
    let user = std::env::var("SMTP_USER").unwrap_or_default();
    let password = std::env::var("SMTP_PASSWORD").unwrap_or_default();
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@poziomki.app".into());
    let tls_name = std::env::var("SMTP_TLS_NAME").unwrap_or_else(|_| host.clone());
    SmtpSettings {
        host,
        port,
        user,
        password,
        from,
        tls_name,
    }
}

fn parse_from_mailbox(from: &str) -> Option<Mailbox> {
    let Ok(from_addr) = from.parse() else {
        tracing::error!("Invalid SMTP_FROM address: {from}");
        return None;
    };
    Some(Mailbox::new(
        Some("poziomki – poznajmy się!".to_string()),
        from_addr,
    ))
}

fn parse_recipient_mailbox(to: &str) -> Option<Mailbox> {
    let Ok(to_addr) = to.parse() else {
        tracing::error!("Invalid recipient address: {to}");
        return None;
    };
    Some(to_addr)
}

fn build_otp_email(from_mbox: Mailbox, to_mbox: Mailbox, code: &str, to: &str) -> Option<Message> {
    let html_body = otp_email_html(code);
    let plain_body = format!(
        "Twój kod logowania: {code}\n\nWpisz ten kod w aplikacji, aby potwierdzić swoje konto.\nKod wygasa za 10 minut.\n\n2026 poziomki 🩵"
    );
    let msg_id = format!("<{}@poziomki.app>", uuid::Uuid::new_v4());

    match Message::builder()
        .from(from_mbox)
        .to(to_mbox)
        .message_id(Some(msg_id))
        .subject(format!("Twój kod logowania to {code}"))
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
            Some(msg)
        }
        Err(error) => {
            tracing::error!("Failed to build OTP email: {error}");
            None
        }
    }
}

fn smtp_tls_params(tls_name: &str) -> Option<TlsParameters> {
    match TlsParameters::new(tls_name.to_string()) {
        Ok(params) => Some(params),
        Err(error) => {
            tracing::error!("Failed to create TLS parameters for {tls_name}: {error}");
            None
        }
    }
}

fn build_smtp_mailer(
    settings: &SmtpSettings,
    tls_params: TlsParameters,
) -> AsyncSmtpTransport<Tokio1Executor> {
    let creds = Credentials::new(settings.user.clone(), settings.password.clone());
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
        .port(settings.port)
        .tls(lettre::transport::smtp::client::Tls::Required(tls_params))
        .credentials(creds)
        .build()
}

pub(super) async fn send_otp_email(to: &str, code: &str) {
    if !env_truthy("SMTP_ENABLE") {
        tracing::debug!("SMTP disabled, skipping OTP email to {to}");
        return;
    }

    let settings = smtp_settings();
    let Some(from_mbox) = parse_from_mailbox(&settings.from) else {
        return;
    };
    let Some(to_mbox) = parse_recipient_mailbox(to) else {
        return;
    };
    let Some(email) = build_otp_email(from_mbox, to_mbox, code, to) else {
        return;
    };
    let Some(tls_params) = smtp_tls_params(&settings.tls_name) else {
        return;
    };
    let mailer = build_smtp_mailer(&settings, tls_params);

    if let Err(error) = mailer.send(email).await {
        tracing::error!("Failed to send OTP email to {to}: {error}");
    } else {
        tracing::info!("OTP email sent to {to}");
    }
}

pub(super) async fn verify_otp_inner(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    email: &str,
    otp: &str,
) -> std::result::Result<Response, crate::error::AppError> {
    let Some(user) = find_user_by_email(db, email).await? else {
        return Ok(invalid_otp_response(headers));
    };

    let otp_ok = verify_otp_db(db, email, otp).await;
    if !otp_ok {
        return Ok(invalid_otp_response(headers));
    }

    let mut verified_user = user.clone();
    if verified_user.email_verified_at.is_none() {
        let mut active: users::ActiveModel = user.clone().into();
        let now = Utc::now();
        active.email_verified_at = sea_orm::ActiveValue::Set(Some(now.into()));
        let _ = active.update(db).await;
        verified_user.email_verified_at = Some(now.into());
    }

    let session = create_session_db(db, headers, verified_user.id).await?;
    Ok(Json(DataResponse {
        data: serde_json::json!({
            "user": user_model_to_view(&verified_user),
            "token": session.token,
            "session": session_model_to_view(&session.model),
            "status": true,
        }),
    })
    .into_response())
}
