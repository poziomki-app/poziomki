use axum::http::{HeaderMap, StatusCode};
use loco_rs::prelude::*;
use opendal::ErrorKind;

use super::super::{
    error_response,
    state::{
        is_chat_context, is_upload_public, lock_state, require_auth, validate_filename,
        MigrationState,
    },
    ErrorSpec,
};
use super::{uploads_multipart::HandlerResult, uploads_storage, uploads_storage::StorageError};

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

pub(super) fn can_access_upload(state: &MigrationState, filename: &str, profile_id: &str) -> bool {
    state
        .uploads
        .get(filename)
        .filter(|record| !record.deleted)
        .is_some_and(|record| {
            is_upload_public(record.context)
                || (is_chat_context(record.context)
                    && (record.owner_id.as_deref() == Some(profile_id)
                        || record.context_id.as_ref().is_some_and(|context_id| {
                            state
                                .event_attendees
                                .contains_key(&(context_id.clone(), profile_id.to_string()))
                        })))
        })
}

fn extract_upload_filename(original_uri: &str) -> Option<&str> {
    original_uri
        .split('?')
        .next()
        .unwrap_or(original_uri)
        .strip_prefix("/uploads/")
}

fn original_uri_from_headers(headers: &HeaderMap) -> HandlerResult<&str> {
    headers
        .get("x-original-uri")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            Box::new(bad_request(
                headers,
                "MISSING_URI",
                "X-Original-URI header required",
            ))
        })
}

pub(super) fn filename_from_headers(headers: &HeaderMap) -> HandlerResult<String> {
    let original_uri = original_uri_from_headers(headers)?;
    extract_upload_filename(original_uri)
        .filter(|filename| !filename.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            Box::new(bad_request(
                headers,
                "INVALID_URI",
                "Invalid upload URI format",
            ))
        })
}

fn storage_error_to_response(headers: &HeaderMap, err: &StorageError) -> Response {
    match err.kind {
        Some(ErrorKind::NotFound) => not_found(headers),
        _ => internal_error(headers, "Upload storage is unavailable"),
    }
}

pub(super) async fn storage_exists(headers: &HeaderMap, filename: &str) -> HandlerResult<bool> {
    uploads_storage::exists(filename)
        .await
        .map_err(|err| Box::new(storage_error_to_response(headers, &err)))
}

pub(super) async fn storage_signed_url(
    headers: &HeaderMap,
    filename: &str,
) -> HandlerResult<String> {
    uploads_storage::signed_get_url(filename)
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

pub(super) fn ensure_can_delete_upload(headers: &HeaderMap, filename: &str) -> HandlerResult<()> {
    if let Err(message) = validate_filename(filename) {
        return Err(Box::new(bad_request(headers, "INVALID_FILENAME", message)));
    }

    let exists = {
        let mut state = lock_state();
        require_auth(headers, &mut state)?;
        state
            .uploads
            .get_mut(filename)
            .filter(|record| !record.deleted)
            .is_some()
    };
    if exists {
        Ok(())
    } else {
        Err(Box::new(not_found(headers)))
    }
}

pub(super) fn mark_upload_deleted(headers: &HeaderMap, filename: &str) -> HandlerResult<()> {
    let mut state = lock_state();
    let record = state
        .uploads
        .get_mut(filename)
        .filter(|record| !record.deleted)
        .ok_or_else(|| Box::new(not_found(headers)))?;
    record.deleted = true;
    drop(state);
    Ok(())
}
