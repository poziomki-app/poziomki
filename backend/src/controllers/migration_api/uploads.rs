#[path = "uploads_multipart.rs"]
mod uploads_multipart;

use axum::{
    extract::{Multipart, Path},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use loco_rs::prelude::*;

use super::{
    error_response,
    state::{
        create_upload_filename, is_chat_context, is_production_mode, is_upload_public, lock_state,
        require_auth, require_profile, validate_filename, AuthCheckResponse, MigrationState,
        SuccessResponse, UploadRecord, UploadResponse, UploadUrlResponse,
    },
    ErrorSpec,
};
use uploads_multipart::{read_multipart, HandlerError};

fn bad_request(headers: &HeaderMap, code: &'static str, message: &str) -> Response {
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

fn not_found(headers: &HeaderMap) -> Response {
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

fn forbidden(headers: &HeaderMap, code: &'static str, message: &str) -> Response {
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

fn can_access_upload(state: &MigrationState, filename: &str, profile_id: &str) -> bool {
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

fn original_uri_from_headers(headers: &HeaderMap) -> std::result::Result<&str, HandlerError> {
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

fn filename_from_headers(headers: &HeaderMap) -> std::result::Result<String, HandlerError> {
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

fn production_file_get(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<Response, HandlerError> {
    let mut state = lock_state();
    require_auth(headers, &mut state)?;

    if state
        .uploads
        .get(filename)
        .is_none_or(|record| record.deleted)
    {
        drop(state);
        return Err(Box::new(not_found(headers)));
    }
    drop(state);

    Ok(Json(UploadUrlResponse {
        url: format!("/api/v1/uploads/{filename}"),
    })
    .into_response())
}

fn development_file_get(
    filename: &str,
    headers: &HeaderMap,
) -> std::result::Result<Response, HandlerError> {
    let state = lock_state();
    let record = state
        .uploads
        .get(filename)
        .filter(|record| !record.deleted)
        .ok_or_else(|| Box::new(not_found(headers)))?;

    let bytes = state
        .upload_blobs
        .get(filename)
        .ok_or_else(|| Box::new(not_found(headers)))?
        .clone();
    let mime_type = record.mime_type.clone();
    drop(state);

    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000"),
    );

    Ok(response)
}

fn file_get_impl(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<Response, HandlerError> {
    validate_filename(filename)
        .map_err(|message| Box::new(bad_request(headers, "INVALID_FILENAME", message)))?;

    if is_production_mode() {
        production_file_get(headers, filename)
    } else {
        development_file_get(filename, headers)
    }
}

fn auth_check_impl(headers: &HeaderMap) -> std::result::Result<Response, HandlerError> {
    let mut state = lock_state();
    let (_session, user) = require_auth(headers, &mut state)?;
    let profile = require_profile(headers, &state, &user.id)?;

    let filename = filename_from_headers(headers)?;
    if !can_access_upload(&state, &filename, &profile.id) {
        drop(state);
        return Err(Box::new(forbidden(
            headers,
            "ACCESS_DENIED",
            "You cannot access this file",
        )));
    }
    drop(state);

    Ok(Json(AuthCheckResponse { ok: true }).into_response())
}

pub(super) async fn auth_check(headers: HeaderMap) -> Result<Response> {
    Ok(auth_check_impl(&headers).unwrap_or_else(|response| *response))
}

pub(super) async fn file_get(headers: HeaderMap, Path(filename): Path<String>) -> Result<Response> {
    Ok(file_get_impl(&headers, &filename).unwrap_or_else(|response| *response))
}

pub(super) async fn file_upload(headers: HeaderMap, multipart: Multipart) -> Result<Response> {
    let owner_id = {
        let mut state = lock_state();
        let (_session, user) = match require_auth(&headers, &mut state) {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };
        state.profiles_by_user.get(&user.id).cloned()
    };

    let parsed = match read_multipart(&headers, multipart).await {
        Ok(parsed) => parsed,
        Err(response) => return Ok(*response),
    };

    let filename = create_upload_filename(&parsed.mime_type);
    {
        let mut state = lock_state();
        state
            .upload_blobs
            .insert(filename.clone(), parsed.bytes.clone());
        state.uploads.insert(
            filename.clone(),
            UploadRecord {
                owner_id,
                context: parsed.context,
                context_id: parsed.context_id,
                mime_type: parsed.mime_type.clone(),
                deleted: false,
            },
        );
    }

    Ok(Json(UploadResponse {
        url: format!("/api/v1/uploads/{filename}"),
        filename,
        size: parsed.bytes.len(),
        mime_type: parsed.mime_type,
    })
    .into_response())
}

fn file_delete_impl(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<Response, HandlerError> {
    validate_filename(filename)
        .map_err(|message| Box::new(bad_request(headers, "INVALID_FILENAME", message)))?;

    let mut state = lock_state();
    require_auth(headers, &mut state)?;

    let record = state
        .uploads
        .get_mut(filename)
        .filter(|record| !record.deleted)
        .ok_or_else(|| Box::new(not_found(headers)))?;

    record.deleted = true;
    state.upload_blobs.remove(filename);
    drop(state);
    Ok(Json(SuccessResponse { success: true }).into_response())
}

pub(super) async fn file_delete(
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    Ok(file_delete_impl(&headers, &filename).unwrap_or_else(|response| *response))
}
