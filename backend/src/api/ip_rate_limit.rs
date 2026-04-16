//! IP-keyed rate limits for non-auth endpoints.
//!
//! This mirrors the DB-backed limiter in `api::auth::rate_limit` but keys on
//! the client IP from the reverse proxy instead of the per-user subject.
//!
//! **Header choice matters for exploitability.** Caddy's default
//! `reverse_proxy` behaviour *appends* the client IP to any existing
//! `X-Forwarded-For`, which means an attacker who sends
//! `X-Forwarded-For: 1.2.3.4` can control the first hop. So we prefer
//! `X-Real-IP`, which Caddy sets authoritatively on the API hosts
//! (`infra/caddy/Caddyfile.prod` configures `header_up X-Real-IP
//! {client_ip}`). We still read XFF as a fallback, but take the *last* hop —
//! the one Caddy appended — rather than the first, so a spoofed XFF value
//! can't bypass the limit.
use axum::http::HeaderMap;
use axum::response::Response;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::Integer;
use diesel_async::RunQueryDsl;

use super::{error_response, ErrorSpec};

const IP_RATE_LIMIT_WINDOW_SECS: i64 = 60;
const MATCHING_PROFILES_MAX_PER_MIN: u32 = 30;
const CHAT_WS_UPGRADE_MAX_PER_MIN: u32 = 60;

#[derive(Clone, Copy, Debug)]
pub enum IpRateLimitAction {
    MatchingProfiles,
    ChatWsUpgrade,
}

impl IpRateLimitAction {
    const fn max_attempts(self) -> u32 {
        match self {
            Self::MatchingProfiles => MATCHING_PROFILES_MAX_PER_MIN,
            Self::ChatWsUpgrade => CHAT_WS_UPGRADE_MAX_PER_MIN,
        }
    }

    const fn key_prefix(self) -> &'static str {
        match self {
            Self::MatchingProfiles => "ip_matching_profiles",
            Self::ChatWsUpgrade => "ip_chat_ws_upgrade",
        }
    }
}

/// Resolve the client IP from proxy headers, in order of trustworthiness:
///
/// 1. `X-Real-IP` — Caddy sets this authoritatively; no client input can
///    forge it past the proxy boundary.
/// 2. `X-Forwarded-For` last hop — Caddy appends, so the rightmost value is
///    what Caddy saw. Reading the first hop would be spoofable.
/// 3. `"unknown"` — a single shared bucket, so the endpoint is still capped
///    when headers are missing.
fn client_ip(headers: &HeaderMap) -> String {
    if let Some(value) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(last) = value.split(',').next_back() {
            let trimmed = last.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "unknown".to_string()
}

fn rate_limit_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        headers,
        ErrorSpec {
            error: "Too many requests, try again later".to_string(),
            code: "RATE_LIMITED",
            details: None,
        },
    )
}

#[derive(QueryableByName)]
struct AttemptRow {
    #[diesel(sql_type = Integer)]
    attempts: i32,
}

async fn upsert_attempt(
    key: &str,
    window_secs: i64,
) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = crate::db::conn().await?;
    let row = diesel::sql_query(
        r"
        INSERT INTO auth_rate_limits (id, rate_key, window_start, attempts, updated_at)
        VALUES (gen_random_uuid(), $1, NOW(), 1, NOW())
        ON CONFLICT (rate_key) DO UPDATE
        SET
            window_start = CASE
                WHEN auth_rate_limits.window_start <= NOW() - make_interval(secs => $2)
                    THEN NOW()
                ELSE auth_rate_limits.window_start
            END,
            attempts = CASE
                WHEN auth_rate_limits.window_start <= NOW() - make_interval(secs => $2)
                    THEN 1
                ELSE auth_rate_limits.attempts + 1
            END,
            updated_at = NOW()
        RETURNING attempts
        ",
    )
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::BigInt, _>(window_secs)
    .get_result::<AttemptRow>(&mut conn)
    .await?;

    Ok(i64::from(row.attempts))
}

/// Throttle requests by (action, client IP).
///
/// Returns `Err(response)` with a 429 body when the caller exceeds the
/// action's per-minute cap. Fails open on DB errors so a database hiccup
/// doesn't take the API offline.
pub async fn enforce_ip_rate_limit(
    headers: &HeaderMap,
    action: IpRateLimitAction,
) -> std::result::Result<(), Box<Response>> {
    let ip = client_ip(headers);
    let key = format!("{}:{ip}", action.key_prefix());

    let attempts = match upsert_attempt(&key, IP_RATE_LIMIT_WINDOW_SECS).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%error, rate_key = %key, "ip rate limiter unavailable; allowing request");
            return Ok(());
        }
    };

    if attempts > i64::from(action.max_attempts()) {
        Err(Box::new(rate_limit_response(headers)))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::client_ip;
    use axum::http::HeaderMap;

    fn headers_with(name: &'static str, value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(name, value.parse().expect("valid header value"));
        h
    }

    #[test]
    fn prefers_x_real_ip_over_xff() {
        // If Caddy sets both (which it does), X-Real-IP wins because it
        // isn't appended to by convention and can't be spoofed past Caddy.
        let mut h = HeaderMap::new();
        h.insert(
            "x-real-ip",
            "198.51.100.7".parse().expect("valid header value"),
        );
        h.insert(
            "x-forwarded-for",
            "1.2.3.4, 10.0.0.1".parse().expect("valid header value"),
        );
        assert_eq!(client_ip(&h), "198.51.100.7");
    }

    #[test]
    fn uses_xff_last_hop_when_no_real_ip() {
        // A malicious client sending X-Forwarded-For: 1.2.3.4 gets appended
        // to by Caddy, so the last hop is the one the proxy observed.
        let h = headers_with("x-forwarded-for", "1.2.3.4, 203.0.113.9");
        assert_eq!(client_ip(&h), "203.0.113.9");
    }

    #[test]
    fn handles_single_xff_hop() {
        let h = headers_with("x-forwarded-for", "203.0.113.5");
        assert_eq!(client_ip(&h), "203.0.113.5");
    }

    #[test]
    fn falls_back_to_unknown_bucket() {
        assert_eq!(client_ip(&HeaderMap::new()), "unknown");
    }
}
