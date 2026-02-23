use axum::http::HeaderMap;

use super::uploads_http::bad_request;
use super::uploads_multipart::HandlerError;
use crate::api::state::{
    allowed_upload_mime, is_chat_context, is_s3_storage_configured, max_upload_size_bytes,
    parse_upload_context, validate_filename, DirectUploadPresignBody, UploadContext,
};

pub(super) fn extract_filename_from_original_uri(
    headers: &HeaderMap,
) -> std::result::Result<String, HandlerError> {
    let original_uri = headers
        .get("x-original-uri")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            Box::new(bad_request(
                headers,
                "MISSING_URI",
                "Missing X-Original-URI header",
            )) as HandlerError
        })?;

    let path = original_uri.split('?').next().unwrap_or_default();
    let filename = path
        .strip_prefix("/uploads/")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            Box::new(bad_request(headers, "INVALID_URI", "Invalid upload URI")) as HandlerError
        })?;

    if let Err(message) = validate_filename(filename) {
        return Err(Box::new(bad_request(headers, "INVALID_FILENAME", message)));
    }

    Ok(filename.to_string())
}

pub(super) fn validate_presign_payload(
    headers: &HeaderMap,
    payload: &DirectUploadPresignBody,
) -> std::result::Result<UploadContext, HandlerError> {
    if !is_s3_storage_configured() {
        return Err(Box::new(bad_request(
            headers,
            "DIRECT_UPLOAD_UNAVAILABLE",
            "Direct upload presign is available only in production/Garage mode",
        )));
    }
    let context = parse_upload_context(payload.context.as_deref()).ok_or_else(|| {
        Box::new(bad_request(
            headers,
            "VALIDATION_ERROR",
            "Invalid upload context",
        )) as HandlerError
    })?;
    validate_presign_fields(headers, payload, context)?;
    Ok(context)
}

pub(super) fn validate_presign_fields(
    headers: &HeaderMap,
    payload: &DirectUploadPresignBody,
    context: UploadContext,
) -> std::result::Result<(), HandlerError> {
    if is_chat_context(context) && payload.context_id.as_deref().is_none_or(str::is_empty) {
        return Err(Box::new(bad_request(
            headers,
            "MISSING_CONTEXT_ID",
            "contextId required for chat uploads",
        )));
    }
    check_upload_constraints(headers, &payload.mime_type, payload.size)
}

pub(super) fn check_upload_constraints(
    headers: &HeaderMap,
    mime_type: &str,
    size: usize,
) -> std::result::Result<(), HandlerError> {
    if !allowed_upload_mime(mime_type) {
        return Err(Box::new(bad_request(
            headers,
            "INVALID_FILE_TYPE",
            "Allowed: image/jpeg, image/png, image/webp",
        )));
    }
    if size == 0 || size > max_upload_size_bytes() {
        return Err(Box::new(bad_request(
            headers,
            "FILE_TOO_LARGE",
            "Max: 10MB",
        )));
    }
    Ok(())
}
