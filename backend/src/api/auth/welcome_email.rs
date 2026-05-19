//! Welcome email for pre-launch (early-access) signups.
//!
//! Triggered after the landing-page onboarding flow completes its final
//! step. Unlike OTP delivery — which goes through the full relay → Resend
//! → MX cascade because it's on the critical path for account access —
//! the welcome email is best-effort: a missed welcome email is annoying,
//! not blocking, so we use Resend only. If `RESEND_API_KEY` is unset
//! (local dev), the job is a no-op.
//!
//! The job is idempotent at the DB layer (`users.welcome_email_sent_at`
//! only ever moves from NULL to a timestamp once), so a retry of the
//! enqueue path never produces a duplicate inbox delivery.

// The outbox topic that consumes this module lives on a sibling branch
// not yet merged to main, so the helpers below appear unused from main
// until the worker integration lands.
#![allow(dead_code)]

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

const RESEND_URL: &str = "https://api.resend.com/emails";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Render the welcome-email HTML body. Plain, single-CTA, mobile-first.
/// Kept inline so this module has zero file-template dependencies.
fn welcome_email_html(display_name: &str) -> String {
    let safe_name = html_escape(display_name);
    format!(
        r#"<!doctype html>
<html lang="pl">
<head><meta charset="utf-8" /><title>Cześć, {safe_name}!</title></head>
<body style="margin:0;padding:0;background:#0e1110;font-family:system-ui,-apple-system,sans-serif;color:#f2eee6;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:#0e1110;">
    <tr><td align="center" style="padding:32px 16px;">
      <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width:520px;background:#171a18;border-radius:24px;padding:32px;">
        <tr><td>
          <h1 style="margin:0 0 16px;font-size:24px;line-height:1.2;color:#f2eee6;">Cześć, {safe_name}!</h1>
          <p style="margin:0 0 16px;font-size:16px;line-height:1.55;color:rgba(242,238,230,0.85);">Twoje konto wczesnego dostępu jest gotowe. Odezwiemy się, jak tylko otworzymy testy zamknięte — przed oficjalnym startem aplikacji w lipcu 2026.</p>
          <p style="margin:0 0 16px;font-size:16px;line-height:1.55;color:rgba(242,238,230,0.85);">Do tego czasu nie musisz nic robić. Trzymaj kciuki i daj znać znajomym, że szykuje się coś fajnego 🍓</p>
          <p style="margin:24px 0 0;font-size:14px;color:rgba(242,238,230,0.55);">— ekipa poziomek</p>
        </td></tr>
      </table>
    </td></tr>
  </table>
</body></html>"#
    )
}

fn welcome_email_text(display_name: &str) -> String {
    format!(
        "Cześć, {display_name}!\n\n\
         Twoje konto wczesnego dostępu jest gotowe. Odezwiemy się, jak tylko \
         otworzymy testy zamknięte — przed oficjalnym startem aplikacji w lipcu 2026.\n\n\
         Do tego czasu nie musisz nic robić. Trzymaj kciuki i daj znać znajomym, \
         że szykuje się coś fajnego 🍓\n\n\
         — ekipa poziomek"
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

#[derive(Debug, Deserialize)]
struct ResendError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

/// Deliver the welcome email via Resend. Returns `Ok(())` on success or
/// when delivery is skipped (`RESEND_API_KEY` unset). Returns `Err` only
/// for transport / API failures the worker should retry.
pub(in crate::api) async fn send_welcome_email(to: &str, display_name: &str) -> Result<(), String> {
    let Some(api_key) = env_nonempty("RESEND_API_KEY") else {
        tracing::debug!(
            email = %crate::api::redact_email(to),
            "RESEND_API_KEY unset; skipping welcome email"
        );
        return Ok(());
    };

    let from = env_nonempty("RESEND_FROM")
        .unwrap_or_else(|| "poziomki <noreply@send.poziomki.app>".into());

    let body = json!({
        "from": from,
        "to": [to],
        "subject": "Dzięki za rejestrację — do zobaczenia przed startem",
        "html": welcome_email_html(display_name),
        "text": welcome_email_text(display_name),
        "headers": {
            "Auto-Submitted": "auto-generated",
            "X-Auto-Response-Suppress": "All",
            "Feedback-ID": "welcome:poziomki:poziomki.app",
            "Precedence": "transactional",
        },
    });

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    let response = client
        .post(RESEND_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("resend request: {e}"))?;

    if response.status().is_success() {
        tracing::info!(
            email = %crate::api::redact_email(to),
            "welcome email delivered via Resend"
        );
        return Ok(());
    }

    let status = response.status();
    let parsed: Option<ResendError> = response.json().await.ok();
    let detail = parsed
        .and_then(|e| e.message.or(e.name))
        .unwrap_or_else(|| "no body".into());
    Err(format!("resend {status}: {detail}"))
}
