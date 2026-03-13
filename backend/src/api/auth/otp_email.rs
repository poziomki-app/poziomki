use lettre::{
    message::{header, Mailbox, MultiPart},
    transport::smtp::client::TlsParameters,
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

struct MailSettings {
    from: String,
    dkim_domain: String,
    dkim_selector: String,
    dkim_private_key_pem: String,
}

fn mail_settings() -> MailSettings {
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@poziomki.app".into());
    let dkim_domain = std::env::var("DKIM_DOMAIN").unwrap_or_else(|_| "poziomki.app".into());
    let dkim_selector = std::env::var("DKIM_SELECTOR").unwrap_or_else(|_| "mail".into());
    let dkim_private_key_pem = load_dkim_key();
    MailSettings {
        from,
        dkim_domain,
        dkim_selector,
        dkim_private_key_pem,
    }
}

fn load_dkim_key() -> String {
    if let Ok(path) = std::env::var("DKIM_PRIVATE_KEY_PATH") {
        return std::fs::read_to_string(&path).unwrap_or_else(|e| {
            tracing::error!(path, "failed to read DKIM key file: {e}");
            String::new()
        });
    }
    std::env::var("DKIM_PRIVATE_KEY").unwrap_or_default()
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

fn recipient_domain(to: &str) -> Option<&str> {
    to.rsplit_once('@').map(|(_, domain)| domain)
}

async fn resolve_mx(domain: &str) -> Result<String, String> {
    let resolver = hickory_resolver::Resolver::builder_tokio()
        .map_err(|e| format!("DNS resolver init: {e}"))?
        .build();

    let mx_response = resolver
        .mx_lookup(domain)
        .await
        .map_err(|e| format!("MX lookup failed for {domain}: {e}"))?;

    let mut records: Vec<_> = mx_response.iter().collect();
    records.sort_by_key(|mx| mx.preference());

    records
        .first()
        .filter(|mx| mx.exchange().to_ascii() != ".")
        .map(|mx| {
            let mut host = mx.exchange().to_ascii();
            if host.ends_with('.') {
                host.pop();
            }
            host
        })
        .ok_or_else(|| format!("No MX records for {domain}"))
}

fn parse_dkim_private_key(
    pem: &str,
) -> Result<mail_auth::common::crypto::RsaKey<mail_auth::common::crypto::Sha256>, String> {
    use base64::Engine;
    use mail_auth::common::crypto::{RsaKey, Sha256};

    let is_pkcs1 = pem.contains("RSA PRIVATE KEY");
    let b64: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    let der = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| format!("PEM base64 decode: {e}"))?;

    let key_der = if is_pkcs1 {
        rustls_pki_types::PrivateKeyDer::Pkcs1(der.into())
    } else {
        rustls_pki_types::PrivateKeyDer::Pkcs8(der.into())
    };

    RsaKey::<Sha256>::from_key_der(key_der).map_err(|e| format!("DKIM key parse: {e}"))
}

fn dkim_sign(raw_message: &[u8], settings: &MailSettings) -> Result<Vec<u8>, String> {
    use mail_auth::common::headers::HeaderWriter;

    if settings.dkim_private_key_pem.is_empty() {
        tracing::warn!("No DKIM key configured, sending unsigned");
        return Ok(raw_message.to_vec());
    }

    let pk = parse_dkim_private_key(&settings.dkim_private_key_pem)?;

    let signature = mail_auth::dkim::DkimSigner::from_key(pk)
        .domain(&settings.dkim_domain)
        .selector(&settings.dkim_selector)
        .headers(["From", "To", "Subject", "Date", "Message-ID"])
        .sign(raw_message)
        .map_err(|e| format!("DKIM signing failed: {e}"))?;

    let dkim_header = signature.to_header();
    let mut signed = Vec::with_capacity(dkim_header.len() + raw_message.len());
    signed.extend_from_slice(dkim_header.as_bytes());
    signed.extend_from_slice(raw_message);
    Ok(signed)
}

pub(in crate::api) async fn send_otp_email(to: &str, code: &str) -> Result<(), String> {
    if !super::env_truthy("SMTP_ENABLE") {
        tracing::debug!("Mail disabled, skipping OTP email to {to}");
        return Ok(());
    }

    let settings = mail_settings();
    let from_mbox = parse_from_mailbox(&settings.from).ok_or("Invalid SMTP_FROM address")?;
    let to_mbox =
        parse_recipient_mailbox(to).ok_or_else(|| format!("Invalid recipient address: {to}"))?;
    let email = build_otp_email(from_mbox, to_mbox, code, to).ok_or("Failed to build OTP email")?;

    let envelope = email.envelope().clone();
    let raw = email.formatted();
    let signed = dkim_sign(&raw, &settings)?;

    let domain = recipient_domain(to).ok_or_else(|| format!("Cannot extract domain from {to}"))?;
    let mx_host = resolve_mx(domain).await?;

    let tls_params = TlsParameters::new(mx_host.clone())
        .map_err(|e| format!("TLS params for {mx_host}: {e}"))?;
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mx_host)
        .port(25)
        .tls(lettre::transport::smtp::client::Tls::Opportunistic(
            tls_params,
        ))
        .build();

    mailer
        .send_raw(&envelope, &signed)
        .await
        .map_err(|e| format!("SMTP delivery to {mx_host} failed: {e}"))?;

    tracing::info!("OTP email delivered to {to} via {mx_host}");
    Ok(())
}
