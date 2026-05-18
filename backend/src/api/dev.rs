//! Dev-only endpoints for e2e tests (Maestro flows).
//!
//! All routes are gated on the `POZIOMKI_DEV_TOKEN` env var: missing or empty
//! → 503, mismatched header → 401. The token must be supplied as
//! `X-Dev-Token`. This mirrors the admin gate (see `admin.rs`) so the surface
//! is never exposed accidentally.
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

fn dev_auth(headers: &HeaderMap) -> Result<(), Box<Response>> {
    let Some(expected) = std::env::var("POZIOMKI_DEV_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
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
        tracing::warn!(error = %err, email = %email_lookup, "dev wipe-user failed");
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
