use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::OnceLock;

type HmacSha256 = Hmac<Sha256>;
const SIG_BYTES: usize = 32;

struct ImgproxyConfig {
    hmac_key: Vec<u8>,
    hmac_salt: Vec<u8>,
    base_url: String,
    expiry_secs: u64,
    object_prefix: String,
}

static CONFIG: OnceLock<Option<ImgproxyConfig>> = OnceLock::new();

fn config() -> Option<&'static ImgproxyConfig> {
    CONFIG
        .get_or_init(|| {
            let key_b64 = std::env::var("IMGPROXY_HMAC_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())?;
            let hmac_key = STANDARD.decode(&key_b64).ok()?;
            let salt_b64 = std::env::var("IMGPROXY_HMAC_SALT")
                .ok()
                .filter(|v| !v.trim().is_empty())?;
            let hmac_salt = STANDARD.decode(&salt_b64).ok()?;
            let base_url = std::env::var("IMGPROXY_BASE_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_default();
            let expiry_secs = std::env::var("IMGPROXY_URL_EXPIRY_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120);
            let object_prefix = normalize_prefix(
                std::env::var("IMGPROXY_ALLOWED_PREFIX")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .as_deref()
                    .unwrap_or("uploads/"),
            )?;
            Some(ImgproxyConfig {
                hmac_key,
                hmac_salt,
                base_url,
                expiry_secs,
                object_prefix,
            })
        })
        .as_ref()
}

fn normalize_prefix(raw: &str) -> Option<String> {
    super::common::normalize_object_prefix(raw).ok()
}

pub fn is_configured() -> bool {
    config().is_some()
}

/// Generate a signed imgproxy URL: `{base}/img/{sig}/{expiry}/{variant}.{fmt}/{filename}`
pub fn signed_url(filename: &str, variant: &str, format: &str) -> Option<String> {
    let cfg = config()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let expiry = now + cfg.expiry_secs;
    let object_key = format!("{}{}", cfg.object_prefix, filename);
    let path = format!("{expiry}/{variant}.{format}/{object_key}");
    let sig = sign(&cfg.hmac_key, &cfg.hmac_salt, &path).ok()?;
    Some(format!("{}/img/{sig}/{path}", cfg.base_url))
}

fn sign(key: &[u8], salt: &[u8], path: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(salt);
    mac.update(path.as_bytes());
    let tag = mac.finalize().into_bytes();
    let bytes = tag.get(..SIG_BYTES).ok_or("HMAC tag too short")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
