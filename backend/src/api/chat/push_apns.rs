//! APNs (Apple Push Notification service) HTTP/2 + JWT provider.
//!
//! Sends a wake-up push to iOS devices when a chat message arrives. Like the
//! ntfy path, the payload carries only the conversation id — the client
//! fetches actual content over the authenticated API.
//!
//! Uses token-based auth (.p8 ES256 key) per Apple's modern push flow.

use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::api::common::env_non_empty;

const APNS_PROD_HOST: &str = "https://api.push.apple.com";
const APNS_SANDBOX_HOST: &str = "https://api.sandbox.push.apple.com";
/// Refresh JWT well before APNs' 60-minute hard limit.
const JWT_TTL: Duration = Duration::from_secs(50 * 60);

#[derive(Clone)]
pub struct ApnsConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub key_pem: Vec<u8>,
    pub production: bool,
}

impl ApnsConfig {
    /// Load APNs config from environment. Returns `None` if APNs is not
    /// configured — callers should treat this as "iOS push disabled" without
    /// erroring.
    pub fn from_env() -> Option<Self> {
        let key_id = env_non_empty("APNS_KEY_ID")?;
        let team_id = env_non_empty("APNS_TEAM_ID")?;
        let bundle_id = env_non_empty("APNS_BUNDLE_ID")?;
        let key_path = env_non_empty("APNS_KEY_PATH")?;
        let key_pem = match std::fs::read(&key_path) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(path = %key_path, error = %e, "APNS_KEY_PATH unreadable; iOS push disabled");
                return None;
            }
        };
        let production = env_non_empty("APNS_PRODUCTION")
            .is_none_or(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"));
        Some(Self {
            key_id,
            team_id,
            bundle_id,
            key_pem,
            production,
        })
    }

    const fn host(&self) -> &'static str {
        if self.production {
            APNS_PROD_HOST
        } else {
            APNS_SANDBOX_HOST
        }
    }
}

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    iat: u64,
}

struct CachedJwt {
    token: String,
    issued_at: SystemTime,
}

fn config() -> Option<&'static ApnsConfig> {
    static CFG: OnceLock<Option<ApnsConfig>> = OnceLock::new();
    CFG.get_or_init(ApnsConfig::from_env).as_ref()
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "apns http2 client build failed; using default");
                reqwest::Client::new()
            })
    })
}

#[allow(clippy::significant_drop_tightening)]
async fn current_jwt(cfg: &ApnsConfig) -> Result<String, String> {
    static CACHE: OnceLock<Mutex<Option<CachedJwt>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().await;

    if let Some(cached) = guard.as_ref() {
        if cached.issued_at.elapsed().unwrap_or(Duration::MAX) < JWT_TTL {
            return Ok(cached.token.clone());
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_secs();
    let claims = JwtClaims {
        iss: cfg.team_id.clone(),
        iat: now,
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(cfg.key_id.clone());
    let key = EncodingKey::from_ec_pem(&cfg.key_pem)
        .map_err(|e| format!("apns key parse failed: {e}"))?;
    let token = encode(&header, &claims, &key).map_err(|e| format!("apns jwt sign failed: {e}"))?;

    *guard = Some(CachedJwt {
        token: token.clone(),
        issued_at: SystemTime::now(),
    });
    Ok(token)
}

#[derive(Serialize)]
struct AlertBody<'a> {
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct ApsField<'a> {
    alert: AlertBody<'a>,
    sound: &'a str,
    #[serde(rename = "mutable-content")]
    mutable_content: u8,
}

#[derive(Serialize)]
struct ApnsPayload<'a> {
    aps: ApsField<'a>,
    #[serde(rename = "conversationId")]
    conversation_id: String,
}

pub async fn send_apns(
    apns_token: &str,
    conversation_id: Uuid,
    title: &str,
    body: &str,
) -> Result<(), String> {
    let Some(cfg) = config() else {
        tracing::debug!("apns not configured; skipping ios push");
        return Ok(());
    };
    let jwt = current_jwt(cfg).await?;

    let payload = ApnsPayload {
        aps: ApsField {
            alert: AlertBody { title, body },
            sound: "default",
            mutable_content: 1,
        },
        conversation_id: conversation_id.to_string(),
    };

    let url = format!("{}/3/device/{}", cfg.host(), apns_token);
    let token_prefix: String = apns_token.chars().take(8).collect();
    let resp = http_client()
        .post(&url)
        .header("authorization", format!("bearer {jwt}"))
        .header("apns-topic", &cfg.bundle_id)
        .header("apns-push-type", "alert")
        .header("apns-priority", "10")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("apns request failed: {e}"))?;

    let status = resp.status();
    if status.is_success() {
        tracing::info!(token = token_prefix, "apns_delivered");
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(token = token_prefix, status = %status, body = %body, "apns rejected");
        Err(format!("apns status {status}: {body}"))
    }
}
