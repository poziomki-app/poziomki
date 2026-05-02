/// QR meet-up token: HMAC-SHA256(profile_id:time_bucket, `JWT_SECRET`).
/// Time bucket = `unix_secs` / 300 * 300, so tokens rotate every 5 minutes.
/// We also accept the previous bucket to handle clock skew at rotation boundaries.
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const BUCKET_SECS: u64 = 300; // 5 minutes

fn jwt_secret() -> Vec<u8> {
    std::env::var("JWT_SECRET").unwrap_or_default().into_bytes()
}

const fn bucket_for(secs: u64) -> u64 {
    secs / BUCKET_SECS * BUCKET_SECS
}

fn sign_bucket(profile_id: Uuid, bucket: u64) -> Result<String, String> {
    let secret = jwt_secret();
    let mut mac =
        HmacSha256::new_from_slice(&secret).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(profile_id.as_bytes());
    mac.update(b":");
    mac.update(bucket.to_le_bytes().as_ref());
    let tag = mac.finalize().into_bytes();
    // encode: profile_id (uuid hex, no dashes) + "." + signature
    Ok(format!(
        "{}.{}",
        profile_id.as_simple(),
        URL_SAFE_NO_PAD.encode(tag)
    ))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generate a fresh token for `profile_id`. Returns `(token, expires_at_unix_secs)`.
pub fn generate(profile_id: Uuid) -> Result<(String, u64), String> {
    let now = now_secs();
    let bucket = bucket_for(now);
    let token = sign_bucket(profile_id, bucket)?;
    let expires_at = bucket + BUCKET_SECS;
    Ok((token, expires_at))
}

/// Verify a token. Returns the encoded `profile_id` on success.
/// Accepts both the current and previous bucket to tolerate rotation boundaries.
pub fn verify(token: &str) -> Result<Uuid, &'static str> {
    let (id_part, sig_part) = token.split_once('.').ok_or("invalid token format")?;
    let profile_id = Uuid::parse_str(id_part).map_err(|_| "invalid profile id in token")?;

    let now = now_secs();
    let current = bucket_for(now);
    let previous = current.saturating_sub(BUCKET_SECS);

    for bucket in [current, previous] {
        if let Ok(expected) = sign_bucket(profile_id, bucket) {
            let expected_sig = expected.split_once('.').map_or("", |(_, s)| s);
            if subtle::ConstantTimeEq::ct_eq(sig_part.as_bytes(), expected_sig.as_bytes()).into() {
                return Ok(profile_id);
            }
        }
    }

    Err("invalid or expired token")
}
