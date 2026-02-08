#[path = "uploads_multipart.rs"]
mod uploads_multipart;
#[path = "uploads_storage.rs"]
mod uploads_storage;
#[path = "uploads_support.rs"]
mod uploads_support;

use axum::{
    extract::{Multipart, Path},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use loco_rs::prelude::*;

use super::state::{
    create_upload_filename, is_production_mode, lock_state, require_auth, require_profile,
    validate_filename, AuthCheckResponse, SuccessResponse, UploadRecord, UploadResponse,
    UploadUrlResponse,
};
use uploads_multipart::{read_multipart, HandlerError};
use uploads_support::{
    bad_request, can_access_upload, ensure_can_delete_upload, filename_from_headers, forbidden,
    mark_upload_deleted, not_found, storage_delete, storage_exists, storage_read,
    storage_signed_url, storage_upload,
};

async fn production_file_get(headers: HeaderMap, filename: String) -> Response {
    let response: std::result::Result<Response, HandlerError> = async {
        {
            let mut state = lock_state();
            require_auth(&headers, &mut state)?;
        }

        if !storage_exists(&headers, &filename).await? {
            return Err(Box::new(not_found(&headers)));
        }

        let url = storage_signed_url(&headers, &filename).await?;
        Ok(Json(UploadUrlResponse { url }).into_response())
    }
    .await;

    response.unwrap_or_else(|response| *response)
}

async fn development_file_get(filename: String, headers: HeaderMap) -> Response {
    let mime_type = {
        let state = lock_state();
        state
            .uploads
            .get(&filename)
            .filter(|record| !record.deleted)
            .map(|record| record.mime_type.clone())
    };
    let Some(mime_type) = mime_type else {
        return not_found(&headers);
    };

    let bytes = match storage_read(&headers, &filename).await {
        Ok(value) => value,
        Err(response) => return *response,
    };

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

    response
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

pub(super) fn file_get(
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>> {
    Box::pin(async move {
        if let Err(message) = validate_filename(&filename) {
            return Ok(bad_request(&headers, "INVALID_FILENAME", message));
        }

        if is_production_mode() {
            Ok(production_file_get(headers, filename).await)
        } else {
            Ok(development_file_get(filename, headers).await)
        }
    })
}

pub(super) async fn file_upload(headers: HeaderMap, multipart: Multipart) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let owner_id = {
            let mut state = lock_state();
            let (_session, user) = require_auth(&headers, &mut state)?;
            state.profiles_by_user.get(&user.id).cloned()
        };

        let parsed = read_multipart(&headers, multipart).await?;
        let filename = create_upload_filename(&parsed.mime_type);
        storage_upload(&headers, &filename, &parsed.bytes, &parsed.mime_type).await?;

        {
            let mut state = lock_state();
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

        let url = if is_production_mode() {
            storage_signed_url(&headers, &filename).await?
        } else {
            format!("/api/v1/uploads/{filename}")
        };

        Ok(Json(UploadResponse {
            url,
            filename,
            size: parsed.bytes.len(),
            mime_type: parsed.mime_type,
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|resp| *resp))
}

async fn file_delete_impl(headers: HeaderMap, filename: String) -> Response {
    let response: std::result::Result<Response, HandlerError> = async {
        ensure_can_delete_upload(&headers, &filename)?;
        storage_delete(&headers, &filename).await?;
        mark_upload_deleted(&headers, &filename)?;
        Ok(Json(SuccessResponse { success: true }).into_response())
    }
    .await;

    response.unwrap_or_else(|response| *response)
}

pub(super) async fn file_delete(
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    Ok(file_delete_impl(headers, filename).await)
}
