use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use serde::{Deserialize, Serialize};

use crate::api::state::DataResponse;
use crate::api::{error_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;

use super::{service, token};

type Result<T> = crate::error::AppResult<T>;
type HandlerError = Box<Response>;

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/token", get(get_token))
        .route("/scan", post(scan_token))
        .route("/task", post(claim_task))
}

#[derive(Serialize)]
struct TokenResponse {
    token: String,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
}

#[derive(Serialize)]
struct ScanResponse {
    #[serde(rename = "xpGained")]
    xp_gained: i32,
}

#[derive(Deserialize)]
pub(in crate::api) struct ScanBody {
    token: String,
}

#[derive(Deserialize)]
struct ClaimTaskBody {
    #[serde(rename = "taskId")]
    task_id: String,
}

#[derive(Serialize)]
struct ClaimTaskResponse {
    #[serde(rename = "xpGained")]
    xp_gained: i32,
}

fn unwrap_viewer_tx<T>(
    result: std::result::Result<std::result::Result<T, HandlerError>, diesel::result::Error>,
    headers: &HeaderMap,
) -> std::result::Result<T, HandlerError> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err),
        Err(_) => Err(Box::new(error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Database error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ))),
    }
}

async fn claim_task(headers: HeaderMap, Json(body): Json<ClaimTaskBody>) -> Result<Response> {
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::XpAction,
    )
    .await
    {
        return Ok(*response);
    }

    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    // Load profile + record task completion atomically inside the viewer
    // tx. `try_record_task` is idempotent per (profile_id, task_id, day),
    // so the returned bool decides whether to award XP.
    let headers_tx = headers.clone();
    let task_id = body.task_id.clone();
    let tx_result = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let profile = match service::load_profile_for_user(conn, &headers_tx, user.id).await {
                Ok(p) => p,
                Err(err) => return Ok(Err(err)),
            };
            match service::try_record_task(conn, profile.id, &task_id).await {
                Ok(awarded) => Ok::<_, diesel::result::Error>(Ok((profile.id, awarded))),
                Err(_) => Err(diesel::result::Error::RollbackTransaction),
            }
        }
        .scope_boxed()
    })
    .await;

    let (profile_id, awarded) = match unwrap_viewer_tx(tx_result, &headers) {
        Ok(v) => v,
        Err(resp) => return Ok(*resp),
    };

    if awarded {
        tokio::spawn(async move {
            if let Err(e) = service::award_xp(profile_id, 5).await {
                tracing::warn!(error = %e, %profile_id, "failed to award task XP");
            }
        });
    }

    Ok(Json(DataResponse {
        data: ClaimTaskResponse {
            xp_gained: if awarded { 5 } else { 0 },
        },
    })
    .into_response())
}

pub(in crate::api) async fn get_token(headers: HeaderMap) -> Result<Response> {
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::XpTokenGen,
    )
    .await
    {
        return Ok(*response);
    }

    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let headers_tx = headers.clone();
    let tx_result = db::with_viewer_tx(viewer, move |conn| {
        async move {
            match service::load_profile_for_user(conn, &headers_tx, user.id).await {
                Ok(p) => Ok::<_, diesel::result::Error>(Ok(p)),
                Err(err) => Ok(Err(err)),
            }
        }
        .scope_boxed()
    })
    .await;

    let profile = match unwrap_viewer_tx(tx_result, &headers) {
        Ok(p) => p,
        Err(resp) => return Ok(*resp),
    };

    match token::generate(profile.id) {
        Ok((tok, expires_at)) => Ok(Json(DataResponse {
            data: TokenResponse {
                token: tok,
                expires_at,
            },
        })
        .into_response()),
        Err(e) => {
            tracing::error!(error = %e, "failed to generate XP token");
            Ok(error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "Failed to generate token".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            ))
        }
    }
}

pub(in crate::api) async fn scan_token(
    headers: HeaderMap,
    Json(body): Json<ScanBody>,
) -> Result<Response> {
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::XpAction,
    )
    .await
    {
        return Ok(*response);
    }

    // Authenticate first so an unauthenticated caller can't use a 400
    // "Invalid or expired token" response as an oracle for token validity.
    // The pre-PR flow ran load_profile_for (which called require_auth_db
    // internally) before token::verify — preserve that ordering.
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let Ok(scanned_id) = token::verify(&body.token) else {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Invalid or expired token".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    };

    // Load scanner profile + insert scan record atomically. The scan row
    // is owned by the scanner (scanner_id = viewer's profile), so
    // Tier-A policy on xp_scans can enforce write-on-own-row.
    let headers_tx = headers.clone();
    let tx_result = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let scanner = match service::load_profile_for_user(conn, &headers_tx, user.id).await {
                Ok(p) => p,
                Err(err) => return Ok(Err(err)),
            };
            if scanner.id == scanned_id {
                return Ok(Err(Box::new(error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    &headers_tx,
                    ErrorSpec {
                        error: "Cannot scan your own QR code".to_string(),
                        code: "VALIDATION_ERROR",
                        details: None,
                    },
                ))));
            }
            match service::try_record_scan(conn, scanner.id, scanned_id).await {
                Ok(awarded) => Ok::<_, diesel::result::Error>(Ok((scanner.id, awarded))),
                Err(_) => Err(diesel::result::Error::RollbackTransaction),
            }
        }
        .scope_boxed()
    })
    .await;

    let (my_profile_id, awarded) = match unwrap_viewer_tx(tx_result, &headers) {
        Ok(v) => v,
        Err(resp) => return Ok(*resp),
    };

    if awarded {
        tokio::spawn(async move {
            if let Err(e) = service::award_xp(my_profile_id, SCAN_XP_REWARD).await {
                tracing::warn!(error = %e, profile_id = %my_profile_id, "failed to award XP to scanner");
            }
            if let Err(e) = service::award_xp(scanned_id, SCAN_XP_REWARD).await {
                tracing::warn!(error = %e, profile_id = %scanned_id, "failed to award XP to scanned");
            }
        });
    }

    Ok(Json(DataResponse {
        data: ScanResponse {
            xp_gained: if awarded { SCAN_XP_REWARD } else { 0 },
        },
    })
    .into_response())
}

const SCAN_XP_REWARD: i32 = 25;
