use lettre::{
    message::{
        dkim::{
            dkim_sign, DkimCanonicalization, DkimCanonicalizationType, DkimConfig,
            DkimSigningAlgorithm, DkimSigningKey,
        },
        header, Mailbox, MultiPart,
    },
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
        extension::ClientId,
    },
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
        Ok(Some(DkimConfig::new(
            selector,
            domain,
            key,
            vec![
                header::HeaderName::new_from_ascii_str("From"),
                header::HeaderName::new_from_ascii_str("Subject"),
                header::HeaderName::new_from_ascii_str("To"),
                header::HeaderName::new_from_ascii_str("Date"),
            ],
            DkimCanonicalization {
                header: DkimCanonicalizationType::Relaxed,
                body: DkimCanonicalizationType::Relaxed,
            },
        )))
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
            .and_then(hickory_resolver::ResolverBuilder::build)
            .map_err(|e| format!("DNS resolver: {e}"))
    })
    .as_ref()
    .map_err(std::clone::Clone::clone)
}

fn normalize_mx_host(host: &str) -> String {
    host.strip_suffix('.').unwrap_or(host).to_string()
}

fn select_mx_hosts(domain: &str, records: Vec<(u16, String)>) -> Result<Vec<String>, String> {
    let mut records = records;
    records.sort_by_key(|(preference, _)| *preference);

    let hosts: Vec<String> = records
        .iter()
        .filter(|(_, host)| host != ".")
        .map(|(_, host)| normalize_mx_host(host))
        .collect();

    if hosts.is_empty() {
        return Err(format!("Domain {domain} does not accept email (null MX)"));
    }

    Ok(hosts)
}

async fn resolve_mx(domain: &str) -> Result<Vec<String>, String> {
    let r = resolver()?;
    match r.mx_lookup(domain).await {
        Ok(response) => {
            let records = response
                .answers()
                .iter()
                .filter_map(|record| match &record.data {
                    hickory_resolver::proto::rr::RData::MX(mx) => {
                        Some((mx.preference, mx.exchange.to_ascii()))
                    }
                    _ => None,
                })
                .collect();
            select_mx_hosts(domain, records)
        }
        // RFC 5321 §5: fall back to A/AAAA when the domain has no MX records.
        Err(error) if error.is_no_records_found() => Ok(vec![domain.to_string()]),
        Err(error) => Err(format!("MX lookup failed for {domain}: {error}")),
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

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

async fn send_via_relay(
    envelope: &lettre::address::Envelope,
    body: &[u8],
    relay_host: &str,
) -> Result<(), String> {
    let port: u16 = env_nonempty("SMTP_PORT")
        .and_then(|p| p.parse().ok())
        .unwrap_or(587);
    let tls_name = env_nonempty("SMTP_TLS_NAME").unwrap_or_else(|| relay_host.to_string());
    let ehlo = env_nonempty("SMTP_EHLO").unwrap_or_else(|| "mail.poziomki.app".to_string());

    let tls_params =
        TlsParameters::new(tls_name.clone()).map_err(|e| format!("TLS params {tls_name}: {e}"))?;

    // 465 = implicit TLS, 587 = STARTTLS required, 25 = opportunistic.
    let tls = match port {
        465 => Tls::Wrapper(tls_params),
        25 => Tls::Opportunistic(tls_params),
        _ => Tls::Required(tls_params),
    };

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(relay_host)
        .port(port)
        .hello_name(ClientId::Domain(ehlo))
        .timeout(Some(Duration::from_secs(30)))
        .tls(tls);

    if let (Some(user), Some(pass)) = (env_nonempty("SMTP_USER"), env_nonempty("SMTP_PASSWORD")) {
        builder = builder.credentials(Credentials::new(user, pass));
    }

    let mailer = builder.build();
    mailer
        .send_raw(envelope, body)
        .await
        .map(|_| ())
        .map_err(|e| format!("relay {relay_host}:{port}: {e}"))
}

async fn send_via_resend(to: &str, code: &str) -> Result<(), String> {
    let api_key = env_nonempty("RESEND_API_KEY").ok_or_else(|| "no RESEND_API_KEY".to_string())?;
    let from = env_nonempty("RESEND_FROM").unwrap_or_else(|| {
        // Resend requires the From domain to match a verified domain in their
        // dashboard. send.poziomki.app is the subdomain whose DNS records the
        // dashboard provisioned.
        "poziomki <noreply@send.poziomki.app>".into()
    });

    let body = serde_json::json!({
        "from": from,
        "to": [to],
        "subject": format!("Twój kod logowania to {code}"),
        "html": otp_email_html(code),
        "text": format!(
            "Twój kod logowania: {code}\n\nWpisz ten kod w aplikacji, aby potwierdzić swoje konto.\nKod wygasa za 10 minut.\n\n2026 poziomki 🩵"
        ),
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    let response = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("resend HTTP: {e}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(format!("resend {status}: {text}"))
    }
}

async fn send_via_mx(
    envelope: &lettre::address::Envelope,
    body: &[u8],
    domain: &str,
) -> Result<String, String> {
    let mx_hosts = resolve_mx(domain).await?;
    let ehlo = env_nonempty("SMTP_EHLO").unwrap_or_else(|| "mail.poziomki.app".to_string());
    let mut last_err = String::new();
    for mx_host in &mx_hosts {
        let tls_params = match TlsParameters::new(mx_host.clone()) {
            Ok(p) => p,
            Err(e) => {
                last_err = format!("TLS {mx_host}: {e}");
                continue;
            }
        };
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(mx_host)
            .port(25)
            .hello_name(ClientId::Domain(ehlo.clone()))
            .timeout(Some(Duration::from_secs(30)))
            .tls(Tls::Opportunistic(tls_params))
            .build();
        match mailer.send_raw(envelope, body).await {
            Ok(_) => return Ok(mx_host.clone()),
            Err(e) => last_err = format!("{mx_host}: {e}"),
        }
    }
    Err(format!("All MX hosts failed: {last_err}"))
}

pub(in crate::api) async fn send_otp_email(to: &str, code: &str) -> Result<(), String> {
    if !super::env_truthy("SMTP_ENABLE") {
        tracing::debug!(
            "Mail disabled, skipping OTP email to {}",
            crate::api::redact_email(to)
        );
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
    let redacted = crate::api::redact_email(to);

    // PRIMARY: relay (lettre) — SMTP_HOST + SMTP_USER/SMTP_PASSWORD configured.
    // Most cloud/hosting providers block outbound :25, so direct-to-MX rarely works in prod.
    if let Some(relay_host) = env_nonempty("SMTP_HOST") {
        match send_via_relay(&envelope, &body, &relay_host).await {
            Ok(()) => {
                tracing::info!("OTP email delivered to {redacted} via relay {relay_host}");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("OTP relay failed ({e}); falling back to direct MX for {redacted}");
            }
        }
    }

    // FALLBACK 1: direct-to-MX (works when port 25 outbound is open).
    let domain = to
        .rsplit_once('@')
        .map(|(_, d)| d)
        .ok_or_else(|| format!("No domain in {redacted}"))?;
    let mx_err = match send_via_mx(&envelope, &body, domain).await {
        Ok(mx_host) => {
            tracing::info!("OTP email delivered to {redacted} via MX {mx_host}");
            return Ok(());
        }
        Err(e) => {
            tracing::warn!("OTP direct MX failed ({e}); falling back to Resend for {redacted}");
            e
        }
    };

    // FALLBACK 2: Resend HTTP API. Independent of SMTP/port 25, so it covers
    // VPS network egress problems, MX greylisting, and IP reputation hits.
    // Skipped silently if RESEND_API_KEY is unset — local dev and staging
    // don't need Resend to test the chain.
    match send_via_resend(to, code).await {
        Ok(()) => {
            tracing::info!("OTP email delivered to {redacted} via Resend");
            Ok(())
        }
        Err(resend_err) => Err(format!(
            "OTP delivery failed for {redacted}: MX={mx_err}; resend={resend_err}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::select_mx_hosts;

    #[test]
    fn select_mx_hosts_orders_and_normalizes_records() {
        let hosts = select_mx_hosts(
            "example.com",
            vec![
                (20, "mx2.example.com.".to_string()),
                (10, "mx1.example.com.".to_string()),
            ],
        );

        assert_eq!(
            hosts.ok(),
            Some(vec![
                "mx1.example.com".to_string(),
                "mx2.example.com".to_string()
            ])
        );
    }

    #[test]
    fn select_mx_hosts_rejects_null_mx() {
        let error = select_mx_hosts("example.com", vec![(0, ".".to_string())]).err();
        assert!(matches!(error, Some(error) if error.contains("does not accept email")));
    }
}
