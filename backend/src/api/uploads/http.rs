use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use super::super::{ErrorSpec, error_response};
use super::{
    uploads_multipart::HandlerResult,
    uploads_storage,
    uploads_storage::{StorageError, StorageErrorKind},
};

fn internal_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    )
}

pub(super) fn bad_request(headers: &HeaderMap, code: &'static str, message: &str) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code,
            details: None,
        },
    )
}

pub(super) fn not_found(headers: &HeaderMap) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: "File not found".to_string(),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

pub(super) fn forbidden(headers: &HeaderMap, code: &'static str, message: &str) -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code,
            details: None,
        },
    )
}

fn storage_error_to_response(headers: &HeaderMap, err: &StorageError) -> Response {
    match err.kind {
        Some(StorageErrorKind::NotFound) => not_found(headers),
        _ => internal_error(headers, "Upload storage is unavailable"),
    }
}

pub(super) async fn storage_signed_put_url(
    headers: &HeaderMap,
    filename: &str,
    mime_type: &str,
) -> HandlerResult<String> {
    uploads_storage::signed_put_url(filename, mime_type)
        .await
        .map_err(|err| Box::new(storage_error_to_response(headers, &err)))
}

pub(super) async fn storage_upload(
    headers: &HeaderMap,
    filename: &str,
    bytes: &[u8],
    mime_type: &str,
) -> HandlerResult<()> {
    uploads_storage::upload(filename, bytes, mime_type)
        .await
        .map_err(|err| Box::new(storage_error_to_response(headers, &err)))
}

pub(super) async fn storage_delete(headers: &HeaderMap, filename: &str) -> HandlerResult<()> {
    uploads_storage::delete(filename)
        .await
        .map_err(|err| Box::new(storage_error_to_response(headers, &err)))
}

pub(super) async fn storage_read(headers: &HeaderMap, filename: &str) -> HandlerResult<Vec<u8>> {
    uploads_storage::read(filename)
        .await
        .map_err(|err| Box::new(storage_error_to_response(headers, &err)))
}
