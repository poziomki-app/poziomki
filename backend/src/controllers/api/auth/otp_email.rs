use lettre::{
    message::{header, Mailbox, MultiPart},
    transport::smtp::{authentication::Credentials, client::TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

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

fn prepare_otp_delivery(to: &str, code: &str) -> Option<(Message, SmtpSettings, TlsParameters)> {
    let settings = smtp_settings();
    let from_mbox = parse_from_mailbox(&settings.from)?;
    let to_mbox = parse_recipient_mailbox(to)?;
    let email = build_otp_email(from_mbox, to_mbox, code, to)?;
    let tls_params = smtp_tls_params(&settings.tls_name)?;
    Some((email, settings, tls_params))
}

pub(in crate::controllers::api) async fn send_otp_email(to: &str, code: &str) {
    if !super::env_truthy("SMTP_ENABLE") {
        tracing::debug!("SMTP disabled, skipping OTP email to {to}");
        return;
    }

    let Some((email, settings, tls_params)) = prepare_otp_delivery(to, code) else {
        return;
    };
    let mailer = build_smtp_mailer(&settings, tls_params);

    if let Err(error) = mailer.send(email).await {
        tracing::error!("Failed to send OTP email to {to}: {error}");
    } else {
        tracing::info!("OTP email sent to {to}");
    }
}
