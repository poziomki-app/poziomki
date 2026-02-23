use axum::{http::HeaderMap, response::Response};

use crate::api::{error_response, ErrorSpec};

pub(in crate::api) fn not_found_profile(headers: &HeaderMap, id: &str) -> Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: format!("Profile '{id}' not found"),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

pub(in crate::api) fn validation_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}
