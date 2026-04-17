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
//!
//! **IPv6 bucketing.** IPv6 privacy addresses (RFC 4941) rotate the
//! interface-identifier portion frequently, so a single device can cycle
//! through many distinct /128 addresses within its assigned /64 prefix. We
//! aggregate IPv6 to /64 before hashing into the rate-limit key, so
//! rotation can't slip a single caller past the per-minute cap.
//!
//! **Retry-After.** 429 responses include a `Retry-After` header computed
//! from the current window-start, so well-behaved clients can back off
//! exactly as long as the server expects to stay limited.
use std::net::{IpAddr, Ipv6Addr};

use axum::http::{HeaderMap, HeaderValue};
use axum::response::Response;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::Integer;
use diesel_async::RunQueryDsl;

use super::{error_response, ErrorSpec};

const IP_RATE_LIMIT_WINDOW_SECS: i64 = 60;
const MATCHING_PROFILES_MAX_PER_MIN: u32 = 30;
const CHAT_WS_UPGRADE_MAX_PER_MIN: u32 = 60;
const UPLOAD_WRITE_MAX_PER_MIN: u32 = 30;

/// Bucket name used when no trustworthy client IP can be parsed from the
/// proxy headers. A shared bucket still caps the endpoint, so missing
/// headers don't silently disable throttling.
const UNKNOWN_BUCKET: &str = "unknown";

#[derive(Clone, Copy, Debug)]
pub enum IpRateLimitAction {
    MatchingProfiles,
    ChatWsUpgrade,
    UploadWrite,
}

impl IpRateLimitAction {
    const fn max_attempts(self) -> u32 {
        match self {
            Self::MatchingProfiles => MATCHING_PROFILES_MAX_PER_MIN,
            Self::ChatWsUpgrade => CHAT_WS_UPGRADE_MAX_PER_MIN,
            Self::UploadWrite => UPLOAD_WRITE_MAX_PER_MIN,
        }
    }

    const fn key_prefix(self) -> &'static str {
        match self {
            Self::MatchingProfiles => "ip_matching_profiles",
            Self::ChatWsUpgrade => "ip_chat_ws_upgrade",
            Self::UploadWrite => "ip_upload_write",
        }
    }
}

/// Resolve the client IP from proxy headers, in order of trustworthiness:
///
/// 1. `X-Real-IP` — Caddy sets this authoritatively; no client input can
///    forge it past the proxy boundary.
/// 2. `X-Forwarded-For` last hop — Caddy appends, so the rightmost value is
///    what Caddy saw. Reading the first hop would be spoofable.
///
/// Returns `None` when neither header parses. The caller uses a shared
/// `UNKNOWN_BUCKET` in that case so the endpoint is still capped.
fn client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(value) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Some(ip) = parse_ip(value.trim()) {
            return Some(ip);
        }
    }
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(last) = value.split(',').next_back() {
            if let Some(ip) = parse_ip(last.trim()) {
                return Some(ip);
            }
        }
    }
    None
}

/// Parse a textual IP tolerantly: accept plain `v4`/`v6`, bracketed
/// `[v6]`, and `v4:port` / `[v6]:port`. Any parse error returns `None`.
fn parse_ip(raw: &str) -> Option<IpAddr> {
    if raw.is_empty() {
        return None;
    }
    // [2001:db8::1] or [2001:db8::1]:443
    if let Some(stripped) = raw.strip_prefix('[') {
        let end = stripped.find(']')?;
        return stripped.get(..end)?.parse().ok();
    }
    if let Ok(ip) = raw.parse::<IpAddr>() {
        return Some(ip);
    }
    // "v4:port" — strip a single trailing `:port` iff the remainder has no colons.
    if let Some((host, _)) = raw.rsplit_once(':') {
        if !host.contains(':') {
            return host.parse().ok();
        }
    }
    None
}

/// Reduce an IP to the bucket used as part of the rate-limit key.
///
/// IPv4 keeps its full address. IPv6 collapses to its /64 prefix so
/// privacy-address rotation (RFC 4941) within a single assignment can't
/// be used to slip under the per-IP cap.
fn bucket_key(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            let prefix = Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0);
            format!("{prefix}/64")
        }
    }
}

/// Build the full rate-limit DB key for `(action, caller-bucket)`.
fn rate_key_for(action: IpRateLimitAction, headers: &HeaderMap) -> String {
    let bucket = client_ip(headers).map_or_else(|| UNKNOWN_BUCKET.to_string(), bucket_key);
    format!("{}:{bucket}", action.key_prefix())
}

fn rate_limit_response(headers: &HeaderMap, retry_after_secs: u32) -> Response {
    let mut response = error_response(
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        headers,
        ErrorSpec {
            error: "Too many requests, try again later".to_string(),
            code: "RATE_LIMITED",
            details: None,
        },
    );
    if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        response.headers_mut().insert("retry-after", value);
    }
    response
}

#[derive(QueryableByName)]
struct AttemptRow {
    #[diesel(sql_type = Integer)]
    attempts: i32,
    #[diesel(sql_type = Integer)]
    retry_after_secs: i32,
}

async fn upsert_attempt(
    key: &str,
    window_secs: i64,
) -> std::result::Result<AttemptRow, Box<dyn std::error::Error + Send + Sync>> {
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
        RETURNING
            attempts,
            GREATEST(
                1,
                ($2 - EXTRACT(EPOCH FROM (NOW() - window_start)))::int
            )::int AS retry_after_secs
        ",
    )
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::BigInt, _>(window_secs)
    .get_result::<AttemptRow>(&mut conn)
    .await?;

    Ok(row)
}

/// Throttle requests by (action, client IP /64-bucket).
///
/// Returns `Err(response)` with a 429 body and a `Retry-After` header when
/// the caller exceeds the action's per-minute cap. Fails open on DB errors
/// so a database hiccup doesn't take the API offline — abuse throttling is
/// defense-in-depth, and an unavailable limiter shouldn't also be an
/// outage.
pub async fn enforce_ip_rate_limit(
    headers: &HeaderMap,
    action: IpRateLimitAction,
) -> std::result::Result<(), Box<Response>> {
    let key = rate_key_for(action, headers);

    let row = match upsert_attempt(&key, IP_RATE_LIMIT_WINDOW_SECS).await {
        Ok(value) => value,
        Err(error) => {
            // Intentionally does not log the key: it contains the caller IP,
            // which under attack becomes high-cardinality log noise, and a
            // DB outage signal doesn't need per-IP granularity.
            tracing::warn!(%error, "ip rate limiter unavailable; allowing request");
            return Ok(());
        }
    };

    if i64::from(row.attempts) > i64::from(action.max_attempts()) {
        let retry_after = u32::try_from(row.retry_after_secs.max(1)).unwrap_or(60);
        Err(Box::new(rate_limit_response(headers, retry_after)))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{bucket_key, client_ip, parse_ip, rate_key_for, IpRateLimitAction};
    use axum::http::HeaderMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn headers_with(name: &'static str, value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(name, value.parse().expect("valid header value"));
        h
    }

    #[test]
    fn prefers_x_real_ip_over_xff() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-real-ip",
            "198.51.100.7".parse().expect("valid header value"),
        );
        h.insert(
            "x-forwarded-for",
            "1.2.3.4, 10.0.0.1".parse().expect("valid header value"),
        );
        assert_eq!(
            client_ip(&h),
            Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7)))
        );
    }

    #[test]
    fn uses_xff_last_hop_when_no_real_ip() {
        // A malicious client sending X-Forwarded-For: 1.2.3.4 gets appended
        // to by Caddy, so the last hop is the one the proxy observed.
        let h = headers_with("x-forwarded-for", "1.2.3.4, 203.0.113.9");
        assert_eq!(
            client_ip(&h),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)))
        );
    }

    #[test]
    fn handles_single_xff_hop() {
        let h = headers_with("x-forwarded-for", "203.0.113.5");
        assert_eq!(
            client_ip(&h),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)))
        );
    }

    #[test]
    fn returns_none_when_headers_missing() {
        assert_eq!(client_ip(&HeaderMap::new()), None);
    }

    #[test]
    fn returns_none_on_garbage_header() {
        let h = headers_with("x-real-ip", "not-an-ip");
        assert_eq!(client_ip(&h), None);
    }

    #[test]
    fn parses_bracketed_ipv6() {
        assert_eq!(
            parse_ip("[2001:db8::1]"),
            Some(IpAddr::V6("2001:db8::1".parse().expect("literal")))
        );
    }

    #[test]
    fn parses_bracketed_ipv6_with_port() {
        assert_eq!(
            parse_ip("[2001:db8::1]:443"),
            Some(IpAddr::V6("2001:db8::1".parse().expect("literal")))
        );
    }

    #[test]
    fn parses_ipv4_with_port() {
        assert_eq!(
            parse_ip("203.0.113.9:12345"),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)))
        );
    }

    #[test]
    fn ipv4_bucket_is_full_address() {
        let ip: IpAddr = "203.0.113.9".parse().expect("literal");
        assert_eq!(bucket_key(ip), "203.0.113.9");
    }

    #[test]
    fn ipv6_buckets_to_slash_64() {
        // Two privacy-style addresses in the same /64 should share a bucket.
        let a: IpAddr = IpAddr::V6("2001:db8:abcd:0012:aaaa:bbbb:cccc:dddd".parse().expect("a"));
        let b: IpAddr = IpAddr::V6("2001:db8:abcd:0012:1111:2222:3333:4444".parse().expect("b"));
        let ka = bucket_key(a);
        let kb = bucket_key(b);
        assert_eq!(ka, kb, "same /64 must collapse to one bucket");
        assert_eq!(
            ka,
            format!(
                "{}/64",
                Ipv6Addr::new(0x2001, 0xdb8, 0xabcd, 0x12, 0, 0, 0, 0)
            )
        );
    }

    #[test]
    fn different_ipv6_slash_64_prefixes_are_separate_buckets() {
        let a: IpAddr = IpAddr::V6("2001:db8:abcd:0012::1".parse().expect("a"));
        let b: IpAddr = IpAddr::V6("2001:db8:abcd:0013::1".parse().expect("b"));
        assert_ne!(bucket_key(a), bucket_key(b));
    }

    #[test]
    fn rate_key_falls_back_to_unknown_bucket() {
        // No headers at all → shared "unknown" bucket, still scoped per action.
        let key = rate_key_for(IpRateLimitAction::MatchingProfiles, &HeaderMap::new());
        assert_eq!(key, "ip_matching_profiles:unknown");
    }

    #[test]
    fn rate_key_uses_canonical_ipv4() {
        let h = headers_with("x-real-ip", "198.51.100.7");
        let key = rate_key_for(IpRateLimitAction::ChatWsUpgrade, &h);
        assert_eq!(key, "ip_chat_ws_upgrade:198.51.100.7");
    }

    #[test]
    fn rate_key_uses_ipv6_slash_64() {
        let h = headers_with("x-real-ip", "2001:db8:abcd:0012:aaaa:bbbb:cccc:dddd");
        let key = rate_key_for(IpRateLimitAction::MatchingProfiles, &h);
        assert_eq!(key, "ip_matching_profiles:2001:db8:abcd:12::/64");
    }

    #[test]
    fn rate_key_scopes_upload_write_per_action() {
        let h = headers_with("x-real-ip", "198.51.100.7");
        let key = rate_key_for(IpRateLimitAction::UploadWrite, &h);
        assert_eq!(key, "ip_upload_write:198.51.100.7");
    }
}
