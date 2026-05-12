//! Firebase Cloud Messaging (HTTP v1) push delivery.
//!
//! Privacy model — data-only wake-up.
//!
//! Every payload contains only the conversation UUID and a schema
//! version. The client fetches sender, body, and avatar over the
//! authenticated API after the device wakes up, so Google never sees
//! message content, sender identity, or social graph. On iOS we also
//! include a generic placeholder alert ("Poziomki / Nowa wiadomość")
//! because iOS will not deliver pushes that can become visible without
//! one; a Notification Service Extension replaces it with real content
//! before the user sees it.

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};
use uuid::Uuid;

use crate::db;

const FCM_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";
const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const ACCESS_TOKEN_SKEW: Duration = Duration::from_secs(300);
const SEND_CONCURRENCY: usize = 32;
const PAYLOAD_VERSION: &str = "1";

#[derive(Debug, Clone, Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    refresh_after: Instant,
}

#[derive(Debug)]
pub struct FcmClient {
    project_id: String,
    service_account: ServiceAccount,
    http: reqwest::Client,
    access: RwLock<Option<CachedToken>>,
    sem: Arc<Semaphore>,
}

#[derive(Debug, thiserror::Error)]
pub enum FcmError {
    #[error("FCM is not configured (missing FCM_PROJECT_ID or FCM_SERVICE_ACCOUNT_JSON)")]
    NotConfigured,
    #[error("service account JSON is malformed: {0}")]
    BadServiceAccount(String),
    #[error("JWT signing failed: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("OAuth2 token exchange failed: status {0}")]
    OauthStatus(reqwest::StatusCode),
}

/// Process-wide lazy FCM client; `None` means push is disabled.
pub fn client() -> Option<&'static FcmClient> {
    static CELL: OnceLock<Option<FcmClient>> = OnceLock::new();
    CELL.get_or_init(|| match FcmClient::from_env() {
        Ok(c) => Some(c),
        Err(FcmError::NotConfigured) => {
            tracing::warn!("FCM not configured; push notifications disabled");
            None
        }
        Err(e) => {
            tracing::error!(error = %e, "FCM client init failed; push notifications disabled");
            None
        }
    })
    .as_ref()
}

impl FcmClient {
    fn from_env() -> Result<Self, FcmError> {
        let project_id =
            crate::api::env_non_empty("FCM_PROJECT_ID").ok_or(FcmError::NotConfigured)?;
        let raw =
            crate::api::env_non_empty("FCM_SERVICE_ACCOUNT_JSON").ok_or(FcmError::NotConfigured)?;
        let sa: ServiceAccount =
            serde_json::from_str(&raw).map_err(|e| FcmError::BadServiceAccount(e.to_string()))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            project_id,
            service_account: sa,
            http,
            access: RwLock::new(None),
            sem: Arc::new(Semaphore::new(SEND_CONCURRENCY)),
        })
    }

    async fn access_token(&self) -> Result<String, FcmError> {
        {
            let read = self.access.read().await;
            if let Some(t) = read.as_ref() {
                if Instant::now() < t.refresh_after {
                    return Ok(t.token.clone());
                }
            }
        }
        let mut guard = self.access.write().await;
        if let Some(t) = guard.as_ref() {
            if Instant::now() < t.refresh_after {
                let token = t.token.clone();
                drop(guard);
                return Ok(token);
            }
        }
        let fresh = self.exchange_jwt_for_access_token().await?;
        let token = fresh.token.clone();
        *guard = Some(fresh);
        drop(guard);
        Ok(token)
    }

    async fn exchange_jwt_for_access_token(&self) -> Result<CachedToken, FcmError> {
        #[derive(Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            scope: &'a str,
            aud: &'a str,
            iat: u64,
            exp: u64,
        }
        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: u64,
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let token_uri = self
            .service_account
            .token_uri
            .as_deref()
            .unwrap_or(OAUTH_TOKEN_URL);
        let claims = Claims {
            iss: &self.service_account.client_email,
            scope: FCM_SCOPE,
            aud: token_uri,
            iat: now,
            exp: now + 3600,
        };
        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())?;
        let assertion = encode(&header, &claims, &key)?;
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer")
            .append_pair("assertion", &assertion)
            .finish();
        let resp = self
            .http
            .post(token_uri)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(FcmError::OauthStatus(resp.status()));
        }
        let body: TokenResp = resp.json().await?;
        let lifetime = Duration::from_secs(body.expires_in);
        let refresh_after = Instant::now() + lifetime.saturating_sub(ACCESS_TOKEN_SKEW);
        Ok(CachedToken {
            token: body.access_token,
            refresh_after,
        })
    }

    fn send_url(&self) -> String {
        format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        )
    }
}

/// Build the FCM HTTP v1 message body. Only `conversation_id` + version
/// leave the server — title/body for iOS are a generic placeholder that
/// the Notification Service Extension overwrites client-side before
/// display.
pub(crate) fn build_message(
    token: &str,
    platform: &str,
    conversation_id: Uuid,
) -> serde_json::Value {
    let data = serde_json::json!({
        "conversation_id": conversation_id.to_string(),
        "v": PAYLOAD_VERSION,
    });
    let msg = match platform {
        "ios" => serde_json::json!({
            "token": token,
            "data": data,
            "apns": {
                "headers": { "apns-priority": "10" },
                "payload": {
                    "aps": {
                        "alert": { "title": "Poziomki", "body": "Nowa wiadomość" },
                        "mutable-content": 1,
                        "sound": "default",
                    }
                }
            },
        }),
        _ => serde_json::json!({
            "token": token,
            "data": data,
            "android": {
                "priority": "high",
                "ttl": "600s",
            },
        }),
    };
    serde_json::json!({ "message": msg })
}

fn mask_token(token: &str) -> String {
    let suffix: String = token
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if suffix.chars().count() < 6 {
        return "******".to_string();
    }
    format!("…{suffix}")
}

/// Send a data-only wake-up to every device registered for the given
/// users. Failures are logged but never surfaced — push is best-effort.
pub async fn send_wake(user_ids: Vec<i32>, conversation_id: Uuid) {
    let Some(fcm) = client() else { return };
    let rows = {
        let mut conn = match db::conn().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "fcm: db conn failed");
                return;
            }
        };
        match db::push_tokens_for_users(&mut conn, &user_ids).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "fcm: token lookup failed");
                return;
            }
        }
    };

    let access = match fcm.access_token().await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "fcm: access token failed");
            return;
        }
    };
    let url = fcm.send_url();

    let mut handles = Vec::with_capacity(rows.len());
    for row in rows {
        let sem = fcm.sem.clone();
        let http = fcm.http.clone();
        let access = access.clone();
        let url = url.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.ok();
            let body = build_message(&row.fcm_token, &row.platform, conversation_id);
            let masked = mask_token(&row.fcm_token);
            let resp = http
                .post(&url)
                .bearer_auth(&access)
                .json(&body)
                .send()
                .await;
            match resp {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success() {
                        tracing::info!(token = %masked, "fcm_delivered");
                    } else if status == reqwest::StatusCode::NOT_FOUND
                        || status == reqwest::StatusCode::BAD_REQUEST
                        || status == reqwest::StatusCode::UNAUTHORIZED
                    {
                        let detail = r.text().await.unwrap_or_default();
                        let stale = detail.contains("UNREGISTERED")
                            || detail.contains("INVALID_ARGUMENT")
                            || status == reqwest::StatusCode::NOT_FOUND;
                        tracing::warn!(token = %masked, status = %status, stale, "fcm_rejected");
                        if stale {
                            cleanup_stale_token(&row.fcm_token).await;
                        }
                    } else {
                        tracing::warn!(token = %masked, status = %status, "fcm_error");
                    }
                }
                Err(e) => {
                    tracing::warn!(token = %masked, error = %e, "fcm_send_failed");
                }
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

async fn cleanup_stale_token(fcm_token: &str) {
    use crate::db::schema::push_subscriptions;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let Ok(mut conn) = db::conn().await else {
        return;
    };
    let _ = diesel::delete(
        push_subscriptions::table.filter(push_subscriptions::fcm_token.eq(fcm_token)),
    )
    .execute(&mut conn)
    .await;
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn android_payload_is_data_only() {
        let cid = Uuid::new_v4();
        let v = build_message("tok", "android", cid);
        let msg = v.get("message").unwrap();
        let data = msg.get("data").unwrap();
        assert_eq!(data.as_object().unwrap().len(), 2);
        assert_eq!(data.get("conversation_id").unwrap(), &cid.to_string());
        assert_eq!(data.get("v").unwrap(), PAYLOAD_VERSION);
        assert!(msg.get("notification").is_none());
        assert!(msg.get("apns").is_none());
        assert_eq!(msg.get("android").unwrap().get("priority").unwrap(), "high");
    }

    #[test]
    fn ios_alert_is_generic_placeholder() {
        let cid = Uuid::new_v4();
        let v = build_message("tok", "ios", cid);
        let aps = v.pointer("/message/apns/payload/aps").unwrap();
        let alert = aps.get("alert").unwrap();
        assert_eq!(alert.get("title").unwrap(), "Poziomki");
        assert_eq!(alert.get("body").unwrap(), "Nowa wiadomość");
        assert_eq!(aps.get("mutable-content").unwrap(), 1);
    }

    #[test]
    fn data_block_does_not_leak_pii_keys() {
        let cid = Uuid::new_v4();
        for platform in ["android", "ios"] {
            let v = build_message("tok", platform, cid);
            let data = v.pointer("/message/data").unwrap().to_string();
            for forbidden in ["sender", "body", "preview", "user_id", "username", "title"] {
                assert!(
                    !data.contains(forbidden),
                    "leaked: {forbidden} in data {data}"
                );
            }
        }
    }

    #[test]
    fn mask_token_keeps_only_suffix() {
        assert_eq!(mask_token("abcdefghij"), "…efghij");
        assert_eq!(mask_token("short"), "******");
    }
}
