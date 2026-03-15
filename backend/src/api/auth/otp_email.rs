use lettre::{
    message::{
        dkim::{dkim_sign, DkimConfig, DkimSigningAlgorithm, DkimSigningKey},
        header, Mailbox, MultiPart,
    },
    transport::smtp::client::TlsParameters,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::OnceLock;
use std::time::Duration;

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

// DKIM signing config, loaded once from DKIM_PRIVATE_KEY_PATH.
// Ok(None) = not configured (dev), Ok(Some) = ready, Err = configured but broken.
static DKIM: OnceLock<Result<Option<DkimConfig>, String>> = OnceLock::new();

fn dkim_config() -> Result<Option<&'static DkimConfig>, String> {
    DKIM.get_or_init(|| {
        let Ok(path) = std::env::var("DKIM_PRIVATE_KEY_PATH") else {
            return Ok(None);
        };
        let pem = std::fs::read_to_string(&path).map_err(|e| format!("DKIM key {path}: {e}"))?;
        let key = DkimSigningKey::new(&pem, DkimSigningAlgorithm::Rsa)
            .map_err(|e| format!("DKIM key parse: {e}"))?;
        let domain = std::env::var("DKIM_DOMAIN").unwrap_or_else(|_| "poziomki.app".into());
        let selector = std::env::var("DKIM_SELECTOR").unwrap_or_else(|_| "mail".into());
        Ok(Some(DkimConfig::default_config(selector, domain, key)))
    })
    .as_ref()
    .map(Option::as_ref)
    .map_err(std::clone::Clone::clone)
}

// DNS resolver, initialized once from system config (/etc/resolv.conf).
static DNS: OnceLock<Result<hickory_resolver::TokioResolver, String>> = OnceLock::new();

fn resolver() -> Result<&'static hickory_resolver::TokioResolver, String> {
    DNS.get_or_init(|| {
        hickory_resolver::Resolver::builder_tokio()
            .map(hickory_resolver::ResolverBuilder::build)
            .map_err(|e| format!("DNS resolver: {e}"))
    })
    .as_ref()
    .map_err(std::clone::Clone::clone)
}

async fn resolve_mx(domain: &str) -> Result<Vec<String>, String> {
    let r = resolver()?;
    let hosts: Vec<String> = r.mx_lookup(domain).await.map_or_else(
        |_| vec![],
        |response| {
            let mut records: Vec<_> = response.iter().collect();
            records.sort_by_key(|mx| mx.preference());
            records
                .iter()
                .filter(|mx| mx.exchange().to_ascii() != ".")
                .map(|mx| {
                    let mut h = mx.exchange().to_ascii();
                    if h.ends_with('.') {
                        h.pop();
                    }
                    h
                })
                .collect()
        },
    );
    // RFC 5321 §5: fall back to A/AAAA when no MX records exist
    if hosts.is_empty() {
        return Ok(vec![domain.to_string()]);
    }
    Ok(hosts)
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

fn build_otp_email(from_mbox: Mailbox, to_mbox: Mailbox, code: &str) -> Option<Message> {
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
        Ok(msg) => Some(msg),
        Err(error) => {
            tracing::error!("Failed to build OTP email: {error}");
            None
        }
    }
}

pub(in crate::api) async fn send_otp_email(to: &str, code: &str) -> Result<(), String> {
    if !super::env_truthy("SMTP_ENABLE") {
        tracing::debug!("Mail disabled, skipping OTP email to {to}");
        return Ok(());
    }

    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@poziomki.app".into());
    let from_mbox = parse_from_mailbox(&from).ok_or("Invalid SMTP_FROM")?;
    let to_mbox = parse_recipient_mailbox(to).ok_or_else(|| format!("Invalid recipient: {to}"))?;
    let mut email = build_otp_email(from_mbox, to_mbox, code).ok_or("Failed to build email")?;

    if let Some(dkim) = dkim_config()? {
        dkim_sign(&mut email, dkim);
    }

    let envelope = email.envelope().clone();
    let body = email.formatted();
    let domain = to
        .rsplit_once('@')
        .map(|(_, d)| d)
        .ok_or_else(|| format!("No domain in {to}"))?;
    let mx_hosts = resolve_mx(domain).await?;

    let mut last_err = String::new();
    for mx_host in &mx_hosts {
        let tls_params = match TlsParameters::new(mx_host.clone()) {
            Ok(p) => p,
            Err(e) => {
                last_err = format!("TLS {mx_host}: {e}");
                continue;
            }
        };
        // Opportunistic STARTTLS: maximizes deliverability. OTP has short expiry (10 min)
        // and is rate-limited, so cleartext fallback is an acceptable tradeoff.
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(mx_host)
            .port(25)
            .timeout(Some(Duration::from_secs(30)))
            .tls(lettre::transport::smtp::client::Tls::Opportunistic(
                tls_params,
            ))
            .build();
        match mailer.send_raw(&envelope, &body).await {
            Ok(_) => {
                tracing::info!("OTP email delivered to {to} via {mx_host}");
                return Ok(());
            }
            Err(e) => last_err = format!("{mx_host}: {e}"),
        }
    }
    Err(format!("All MX hosts failed for {to}: {last_err}"))
}
