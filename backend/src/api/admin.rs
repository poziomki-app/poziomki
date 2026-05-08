//! Admin-only endpoints gated on a shared `ADMIN_TOKEN` env var.
//!
//! During invite-only beta the admin surface is intentionally tiny —
//! just account suspension (soft ban) for moderation response. See
//! `.github/MODERATION.md` for the policy this implements.
//!
//! Auth model: the `X-Admin-Token` header must match the
//! `ADMIN_TOKEN` environment variable, compared in constant time.
//! A bearer session is **not** required — operators may be acting
//! without a normal user account. This is a deliberately simple
//! check; the expected caller is a single trusted operator invoking
//! the endpoint from a secure host.
//!
//! The env var is missing → 503. That keeps dev / CI from exposing
//! the endpoint by accident.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::app::AppContext;
use chrono::Utc;
use diesel::sql_types::{Array, Bool, Integer, Nullable, Text, Uuid as SqlUuid};
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::api::{error_response, state, ErrorSpec};

#[derive(diesel::deserialize::QueryableByName)]
struct BanResult {
    #[diesel(sql_type = Nullable<Integer>)]
    user_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BanBody {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SetEventLabelsBody {
    /// Replacement set of labels for the event. An empty list clears them.
    /// Only values from `ALLOWED_EVENT_LABELS` are accepted; everything else
    /// returns 400. Curated server-side so the client surface stays narrow.
    pub labels: Vec<String>,
}

/// Allowlist of valid event labels. Add new entries here and update mobile
/// presentation in tandem; values are stored verbatim in `events.labels`.
const ALLOWED_EVENT_LABELS: &[&str] = &["featured", "partner"];

#[derive(diesel::deserialize::QueryableByName)]
struct LabelsResult {
    #[diesel(sql_type = Bool)]
    found: bool,
}

fn admin_auth(headers: &HeaderMap) -> Result<(), Box<Response>> {
    let Some(expected) = std::env::var("ADMIN_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            headers,
            ErrorSpec {
                error: "Admin endpoint disabled".to_string(),
                code: "ADMIN_DISABLED",
                details: None,
            },
        )));
    };
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Constant-time compare so an attacker probing the header can't
    // derive bytes of the expected token from response timing.
    if expected.as_bytes().ct_eq(provided.as_bytes()).unwrap_u8() != 1 {
        return Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            headers,
            ErrorSpec {
                error: "Admin token rejected".to_string(),
                code: "ADMIN_FORBIDDEN",
                details: None,
            },
        )));
    }
    Ok(())
}

pub(super) async fn ban_user(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(pid): Path<Uuid>,
    Json(body): Json<BanBody>,
) -> Response {
    // Rate limit by caller IP before token compare — caps online
    // brute-force of ADMIN_TOKEN from any single source even though
    // the compare itself is constant time.
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::AdminAuth,
    )
    .await
    {
        return *resp;
    }
    if let Err(resp) = admin_auth(&headers) {
        return *resp;
    }

    let Ok(mut conn) = crate::db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "DB unavailable".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        );
    };

    let now = Utc::now();
    let reason = body.reason.unwrap_or_else(|| "no reason provided".into());

    // The admin request has no viewer context, so a plain UPDATE /
    // DELETE through RLS would filter to zero rows (users policy +
    // sessions policy both require app.current_user_id()). Route
    // through app.admin_ban_user, a SECURITY DEFINER helper that
    // does the pid resolution + UPDATE + session purge in one
    // owner-privileged transaction. Returns the internal user_id
    // (for cache invalidation) or NULL if the pid doesn't exist.
    let ban = diesel::sql_query("SELECT app.admin_ban_user($1, $2) AS user_id")
        .bind::<SqlUuid, _>(pid)
        .bind::<Text, _>(&reason)
        .get_result::<BanResult>(&mut conn)
        .await;

    let user_id = match ban {
        Ok(row) => match row.user_id {
            Some(id) => id,
            None => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &headers,
                    ErrorSpec {
                        error: "User not found".to_string(),
                        code: "NOT_FOUND",
                        details: None,
                    },
                );
            }
        },
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "Ban write failed".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            );
        }
    };

    state::invalidate_auth_cache_for_user_id(user_id).await;
    // Kick any live WebSocket connection belonging to the banned
    // user — the DB + cache path already stops future requests,
    // this closes the last remaining transport on existing sockets.
    ctx.chat_hub.disconnect_user(user_id);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "banned_at": now })),
    )
        .into_response()
}

pub(super) async fn set_event_labels(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(body): Json<SetEventLabelsBody>,
) -> Response {
    if let Err(resp) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::AdminAuth,
    )
    .await
    {
        return *resp;
    }
    if let Err(resp) = admin_auth(&headers) {
        return *resp;
    }

    let mut labels = body.labels;
    for raw in &mut labels {
        *raw = raw.trim().to_string();
    }
    labels.sort();
    labels.dedup();
    if let Some(bad) = labels
        .iter()
        .find(|l| !ALLOWED_EVENT_LABELS.contains(&l.as_str()))
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: format!("Unknown label: {bad}"),
                code: "INVALID_LABEL",
                details: Some(serde_json::json!({ "allowed": ALLOWED_EVENT_LABELS })),
            },
        );
    }

    let Ok(mut conn) = crate::db::conn().await else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "DB unavailable".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        );
    };

    let result = diesel::sql_query("SELECT app.admin_set_event_labels($1, $2) AS found")
        .bind::<SqlUuid, _>(event_id)
        .bind::<Array<Text>, _>(&labels)
        .get_result::<LabelsResult>(&mut conn)
        .await;

    match result {
        Ok(row) if row.found => (
            StatusCode::OK,
            Json(serde_json::json!({ "labels": labels })),
        )
            .into_response(),
        Ok(_) => error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Event not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &headers,
            ErrorSpec {
                error: "Label write failed".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ),
    }
}
