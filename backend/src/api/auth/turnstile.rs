//! Cloudflare Turnstile server-side verification.
//!
//! Used by the landing-page early-access sign-up to gate the public-internet
//! form. The widget on the landing produces a single-use token; we POST it
//! back to Cloudflare with our secret and reject the request if the response
//! isn't `success: true`.
//!
//! Dev / test bypass: when `TURNSTILE_SECRET` is unset (or set to the empty
//! string), verification short-circuits to `Ok`. This keeps local development
//! and the integration test suite from needing a real Cloudflare account.

use std::time::Duration;

use serde::Deserialize;

const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Deserialize)]
struct SiteverifyResponse {
    success: bool,
    #[serde(rename = "error-codes", default)]
    error_codes: Vec<String>,
}

/// Verify a Turnstile token. Returns `Ok(())` when the token is valid or when
/// verification is bypassed in dev. Returns `Err(reason)` with a short
/// classification suitable for logging when verification fails.
///
/// The remote IP is forwarded to Cloudflare when provided — improves their
/// scoring — but is not required.
pub(super) async fn verify_turnstile(token: &str, remote_ip: Option<&str>) -> Result<(), String> {
    let secret = match std::env::var("TURNSTILE_SECRET") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            // No secret configured — local dev / CI. Treat as pass.
            return Ok(());
        }
    };

    if token.trim().is_empty() {
        return Err("missing-token".into());
    }

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    // Manual urlencode — reqwest is built with `default-features = false`
    // so `.form()` isn't available. The three values are short and
    // ASCII-safe in practice, but we percent-encode anyway in case a
    // Turnstile token ever contains a `+` or `=`.
    let mut body = format!(
        "secret={}&response={}",
        percent_encode(&secret),
        percent_encode(token),
    );
    if let Some(ip) = remote_ip {
        body.push_str("&remoteip=");
        body.push_str(&percent_encode(ip));
    }

    let response = client
        .post(SITEVERIFY_URL)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .map_err(|e| format!("siteverify request: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("siteverify status {}", response.status()));
    }

    let parsed: SiteverifyResponse = response
        .json()
        .await
        .map_err(|e| format!("siteverify decode: {e}"))?;

    if parsed.success {
        Ok(())
    } else {
        let reason = parsed
            .error_codes
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        Err(reason)
    }
}

/// Minimal RFC 3986 form-data percent encoder. Encodes everything that
/// isn't an ALPHA / DIGIT / `-_.~`. Avoids pulling in
/// `serde_urlencoded` / `url` for this single call site.
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        let is_safe = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if is_safe {
            out.push(byte as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "%{byte:02X}");
        }
    }
    out
}
