use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::OnceLock;

type HmacSha256 = Hmac<Sha256>;
const SIG_BYTES: usize = 32;
const NONCE_LEN: usize = 24;

struct ImgproxyConfig {
    hmac_key: Vec<u8>,
    hmac_salt: Vec<u8>,
    base_url: String,
    expiry_secs: u64,
    object_prefix: String,
    source_encryption_key: Option<Vec<u8>>,
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
                .unwrap_or(3600);
            let object_prefix = normalize_prefix(
                std::env::var("IMGPROXY_ALLOWED_PREFIX")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .as_deref()
                    .unwrap_or("uploads/"),
            )?;
            let source_encryption_key = std::env::var("IMGPROXY_SOURCE_URL_ENCRYPTION_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .and_then(|b64| STANDARD.decode(&b64).ok())
                .filter(|key| key.len() == 32);
            Some(ImgproxyConfig {
                hmac_key,
                hmac_salt,
                base_url,
                expiry_secs,
                object_prefix,
                source_encryption_key,
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

/// Build the source segment for the URL path (encrypted or plain).
fn source_segment(cfg: &ImgproxyConfig, filename: &str) -> Option<String> {
    let object_key = format!("{}{}", cfg.object_prefix, filename);
    if let Some(enc_key) = &cfg.source_encryption_key {
        encrypt_source_key(enc_key, &object_key).ok()
    } else {
        Some(object_key)
    }
}

/// Generate a signed imgproxy URL: `{base}/img/{sig}/{expiry}/{variant}.{fmt}/{source}`
pub fn signed_url(filename: &str, variant: &str, format: &str) -> Option<String> {
    let cfg = config()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let expiry = now + cfg.expiry_secs;
    let source = source_segment(cfg, filename)?;
    let path = format!("{expiry}/{variant}.{format}/{source}");
    let sig = sign(&cfg.hmac_key, &cfg.hmac_salt, &path).ok()?;
    Some(format!("{}/img/{sig}/{path}", cfg.base_url))
}

/// Generate a signed imgproxy URL for a user's avatar thumbnail.
pub fn signed_avatar_url(filename: &str, format: &str) -> Option<String> {
    signed_url(filename, "thumb", format)
}

fn sign(key: &[u8], salt: &[u8], path: &str) -> Result<String, String> {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(key).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(salt);
    mac.update(path.as_bytes());
    let tag = mac.finalize().into_bytes();
    let bytes = tag.get(..SIG_BYTES).ok_or("HMAC tag too short")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn encrypt_source_key(key: &[u8], object_key: &str) -> Result<String, String> {
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    getrandom::fill(&mut nonce_bytes).map_err(|e| format!("nonce: {e}"))?;
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("cipher: {e}"))?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, object_key.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;
    let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&ciphertext);
    Ok(format!("enc/{}", URL_SAFE_NO_PAD.encode(packed)))
}
