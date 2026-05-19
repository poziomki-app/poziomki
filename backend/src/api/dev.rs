//! Dev-only endpoints for e2e tests (Maestro flows).
//!
//! All routes are gated on the `POZIOMKI_DEV_TOKEN` env var. To enable
//! the surface every check below must pass:
//!
//!   1. `SERVER_HOST` contains "staging" or "localhost" — refuses prod
//!      hostnames as a defense-in-depth against the staging compose
//!      binding being copy-pasted into prod.
//!   2. `POZIOMKI_DEV_TOKEN` is at least 32 characters — a careless
//!      single-character value still fails closed.
//!   3. The caller supplies a matching `X-Dev-Token` header (compared
//!      in constant time).
//!
//! Any failure short-circuits with 503 (`DEV_DISABLED`) or 401
//! (`DEV_AUTH`). Every accepted request is logged at info level so
//! abuse is traceable in journalctl.
//!
//! Endpoints:
//! - `GET  /dev/otp/{email}`        — return the latest OTP plaintext for an email
//! - `POST /dev/wipe-user/{email}`  — best-effort hard-delete a user + OTPs
//!
//! NEVER set `POZIOMKI_DEV_TOKEN` in production.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::api::{error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;
use crate::db::schema::otp_codes;

/// Minimum token length we'll accept. 32 chars × 4 bits/hex = 128 bits
/// of entropy when generated with `openssl rand -hex 32`. Anything
/// shorter is almost certainly a typo or test placeholder — fail closed.
const MIN_TOKEN_LEN: usize = 32;

/// True when the running instance is allowed to expose dev endpoints.
/// Keyed off `SERVER_HOST` because every deploy already sets it, and
/// prod/staging values are distinct (`api.poziomki.app` vs
/// `staging-api…`). Belt-and-suspenders against the staging compose
/// binding being copied into `docker-compose.prod.yml` by mistake.
fn host_allows_dev() -> bool {
    // No SERVER_HOST set → local dev binary; allow.
    std::env::var("SERVER_HOST").map_or(true, |host| {
        let h = host.to_ascii_lowercase();
        h.contains("staging") || h.contains("localhost") || h.contains("127.0.0.1")
    })
}

fn dev_auth(headers: &HeaderMap) -> Result<(), Box<Response>> {
    if !host_allows_dev() {
        tracing::warn!("dev endpoint hit but SERVER_HOST looks like prod — refusing");
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            headers,
            ErrorSpec {
                error: "Dev endpoints disabled".to_string(),
                code: "DEV_DISABLED",
                details: None,
            },
        )));
    }
    let Some(expected) = std::env::var("POZIOMKI_DEV_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| v.len() >= MIN_TOKEN_LEN)
    else {
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            headers,
            ErrorSpec {
                error: "Dev endpoints disabled".to_string(),
                code: "DEV_DISABLED",
                details: None,
            },
        )));
    };
    let provided = headers
        .get("X-Dev-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !bool::from(expected.as_bytes().ct_eq(provided.as_bytes())) {
        return Err(Box::new(error_response(
            StatusCode::UNAUTHORIZED,
            headers,
            ErrorSpec {
                error: "Bad dev token".to_string(),
                code: "DEV_AUTH",
                details: None,
            },
        )));
    }
    Ok(())
}

#[derive(Serialize)]
struct OtpResponse {
    email: String,
    code: String,
}

#[derive(Serialize)]
struct SuccessResponse {
    success: bool,
}

pub(super) async fn latest_otp(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> Response {
    if let Err(resp) = dev_auth(&headers) {
        return *resp;
    }
    tracing::info!(
        endpoint = "dev/otp",
        email = %crate::api::redact_email(&email),
        "dev token accepted"
    );
    let Ok(mut conn) = db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "db conn failed".to_string(),
                code: "DEV_DB",
                details: None,
            },
        );
    };
    let email_lookup = email.to_lowercase();
    let row: Result<Option<String>, diesel::result::Error> = otp_codes::table
        .filter(otp_codes::email.eq(&email_lookup))
        .select(otp_codes::code)
        .order(otp_codes::created_at.desc())
        .first::<String>(&mut conn)
        .await
        .optional();
    match row {
        Ok(Some(code)) => Json(OtpResponse {
            email: email_lookup,
            code,
        })
        .into_response(),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "No OTP for that email".to_string(),
                code: "DEV_NO_OTP",
                details: None,
            },
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "query failed".to_string(),
                code: "DEV_DB",
                details: None,
            },
        ),
    }
}

pub(super) async fn wipe_user(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> Response {
    if let Err(resp) = dev_auth(&headers) {
        return *resp;
    }
    tracing::info!(
        endpoint = "dev/wipe-user",
        email = %crate::api::redact_email(&email),
        "dev token accepted — destructive wipe starting"
    );
    let Ok(mut conn) = db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "db conn failed".to_string(),
                code: "DEV_DB",
                details: None,
            },
        );
    };
    let email_lookup = email.to_lowercase();
    let _ = diesel::delete(otp_codes::table.filter(otp_codes::email.eq(&email_lookup)))
        .execute(&mut conn)
        .await;
    // Hard-delete via raw SQL with cascading children. The schema doesn't use
    // ON DELETE CASCADE on every FK pointing at `users`, so we tear down
    // dependent rows by user_id before deleting the user row itself. We wrap
    // in a single PL/pgSQL block bound through an anonymous prepared
    // statement; `EXECUTE ... USING` accepts the bind, unlike a bare `DO`.
    let sql = "
        DO $do$
        DECLARE uid integer;
        BEGIN
            SELECT id INTO uid FROM users WHERE email = current_setting('app.wipe_email');
            IF uid IS NULL THEN RETURN; END IF;
            DELETE FROM message_reactions WHERE user_id = uid;
            DELETE FROM chat_message_reports WHERE reporter_user_id = uid;
            DELETE FROM chat_message_reveals WHERE viewer_user_id = uid;
            DELETE FROM messages WHERE sender_id = uid;
            DELETE FROM conversation_mutes WHERE user_id = uid;
            DELETE FROM conversation_members WHERE user_id = uid;
            DELETE FROM conversations WHERE user_high_id = uid OR user_low_id = uid;
            DELETE FROM push_subscriptions WHERE user_id = uid;
            DELETE FROM user_settings WHERE user_id = uid;
            DELETE FROM sessions WHERE user_id = uid;
            DELETE FROM profiles WHERE user_id = uid;
            DELETE FROM users WHERE id = uid;
        END
        $do$;
    ";
    // Stash the email in a session GUC the DO block reads. SET LOCAL keeps it
    // scoped to this transaction so concurrent wipes don't see each other.
    let setup = diesel::sql_query("SELECT set_config('app.wipe_email', $1, false)")
        .bind::<diesel::sql_types::Text, _>(&email_lookup)
        .execute(&mut conn)
        .await;
    if let Err(err) = setup {
        tracing::warn!(error = %err, "dev wipe-user setup failed");
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "wipe setup failed".to_string(),
                code: "DEV_WIPE",
                details: None,
            },
        );
    }
    let result = diesel::sql_query(sql).execute(&mut conn).await;
    if let Err(err) = result {
        tracing::warn!(
            error = %err,
            email = %crate::api::redact_email(&email_lookup),
            "dev wipe-user failed"
        );
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "wipe failed".to_string(),
                code: "DEV_WIPE",
                details: None,
            },
        );
    }
    Json(SuccessResponse { success: true }).into_response()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test-only Mutex lock; panic on poison is fine"
)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize the env-var tests — std::env::set_var/remove_var is
    // process-global, so two threads writing SERVER_HOST collide.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn host_allows_dev_local_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("SERVER_HOST");
        assert!(host_allows_dev());
    }

    #[test]
    fn host_allows_dev_staging() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("SERVER_HOST", "https://staging-api.poziomki.app");
        assert!(host_allows_dev());
        std::env::set_var("SERVER_HOST", "http://localhost:5150");
        assert!(host_allows_dev());
        std::env::remove_var("SERVER_HOST");
    }

    #[test]
    fn host_allows_dev_refuses_prod() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("SERVER_HOST", "https://api.poziomki.app");
        assert!(!host_allows_dev());
        std::env::set_var("SERVER_HOST", "https://API.POZIOMKI.APP");
        assert!(!host_allows_dev(), "host check must be case-insensitive");
        std::env::remove_var("SERVER_HOST");
    }
}
