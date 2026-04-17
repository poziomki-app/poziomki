use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::state::DataResponse;
use crate::api::{error_response, ErrorSpec};
use crate::app::AppContext;

use super::{service, token};

type Result<T> = crate::error::AppResult<T>;

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

async fn claim_task(headers: HeaderMap, Json(body): Json<ClaimTaskBody>) -> Result<Response> {
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::XpAction,
    )
    .await
    {
        return Ok(*response);
    }

    let profile = match service::load_profile_for(&headers).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
    };

    let awarded = match service::try_record_task(profile.id, &body.task_id).await {
        Ok(awarded) => awarded,
        Err(e) => {
            tracing::error!(error = %e, "failed to record task completion");
            return Ok(error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "Failed to record task".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            ));
        }
    };

    if awarded {
        let profile_id = profile.id;
        tokio::spawn(async move {
            if let Err(e) = service::award_xp(profile_id, 5).await {
                tracing::warn!(error = %e, profile_id = %profile_id, "failed to award task XP");
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

    let profile = match service::load_profile_for(&headers).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
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

    let scanner = match service::load_profile_for(&headers).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
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

    if scanned_id == scanner.id {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "Cannot scan your own QR code".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let awarded = match service::try_record_scan(scanner.id, scanned_id).await {
        Ok(awarded) => awarded,
        Err(e) => {
            tracing::error!(error = %e, "failed to record XP scan");
            return Ok(error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                &headers,
                ErrorSpec {
                    error: "Failed to record scan".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            ));
        }
    };

    if awarded {
        let my_id = scanner.id;
        tokio::spawn(async move {
            if let Err(e) = service::award_xp(my_id, 5).await {
                tracing::warn!(error = %e, profile_id = %my_id, "failed to award XP to scanner");
            }
            if let Err(e) = service::award_xp(scanned_id, 5).await {
                tracing::warn!(error = %e, profile_id = %scanned_id, "failed to award XP to scanned");
            }
        });
    }

    Ok(Json(DataResponse {
        data: ScanResponse {
            xp_gained: if awarded { 5 } else { 0 },
        },
    })
    .into_response())
}
