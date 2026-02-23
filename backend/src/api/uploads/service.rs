use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::uploads_http_support::{bad_request, forbidden, not_found, storage_signed_url};
use super::uploads_multipart::HandlerError;
use crate::api::state::{
    allowed_upload_mime, is_chat_context, is_s3_storage_configured, max_upload_size_bytes,
    parse_upload_context, require_auth_db, validate_filename, DirectUploadPresignBody,
    UploadContext, UploadUrlResponse,
};
use crate::api::{error_response, ErrorSpec};
use crate::db::models::profiles::Profile;
use crate::db::models::uploads::Upload;
use crate::db::schema::{profiles, uploads};

#[allow(clippy::unnecessary_box_returns)]
pub(super) fn internal_upload_error(headers: &HeaderMap, message: &str) -> HandlerError {
    Box::new(error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    ))
}

fn dev_upload_url(filename: &str) -> String {
    format!("/api/v1/uploads/{filename}")
}

pub(super) async fn public_upload_url(headers: &HeaderMap, filename: &str) -> String {
    if let Some(url) = crate::api::imgproxy_signing::signed_url(filename, "feed", "webp") {
        return url;
    }
    if is_s3_storage_configured() {
        storage_signed_url(headers, filename)
            .await
            .unwrap_or_else(|_| dev_upload_url(filename))
    } else {
        dev_upload_url(filename)
    }
}

pub(super) async fn fallback_variant_urls(
    headers: &HeaderMap,
    original_filename: &str,
) -> (Option<String>, Option<String>) {
    if crate::api::imgproxy_signing::is_configured() {
        let thumb = crate::api::imgproxy_signing::signed_url(original_filename, "thumb", "webp");
        let feed = crate::api::imgproxy_signing::signed_url(original_filename, "feed", "webp");
        return (thumb, feed);
    }
    let fallback = public_upload_url(headers, original_filename).await;
    (Some(fallback.clone()), Some(fallback))
}

pub(super) fn encode_thumbhash(raw: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(raw)
}

pub(super) async fn require_auth_profile(
    headers: &HeaderMap,
) -> std::result::Result<Profile, HandlerError> {
    let (_session, user) = require_auth_db(headers)
        .await
        .map_err(|e| e as HandlerError)?;

    let mut conn = crate::db::conn().await.map_err(|_| {
        Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
    })?;

    profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|_| {
            Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
        })?
        .ok_or_else(|| Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")))
}

pub(super) async fn load_owned_upload(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Upload, HandlerError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?;

    let upload = uploads::table
        .filter(uploads::filename.eq(filename))
        .filter(uploads::deleted.eq(false))
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?
        .ok_or_else(|| Box::new(not_found(headers)) as HandlerError)?;

    if upload.owner_id != Some(owner_profile_id) {
        return Err(Box::new(forbidden(
            headers,
            "ACCESS_DENIED",
            "File access denied",
        )));
    }

    Ok(upload)
}

fn variant_stem(filename: &str) -> Option<&str> {
    filename
        .strip_suffix("_thumb.webp")
        .or_else(|| filename.strip_suffix("_std.webp"))
}

pub(super) async fn load_owned_original_for_variant(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Option<Upload>, HandlerError> {
    let Some(stem) = variant_stem(filename) else {
        return Ok(None);
    };

    let candidates = vec![
        format!("{stem}.jpg"),
        format!("{stem}.jpeg"),
        format!("{stem}.png"),
        format!("{stem}.webp"),
    ];

    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?;

    uploads::table
        .filter(uploads::owner_id.eq(Some(owner_profile_id)))
        .filter(uploads::deleted.eq(false))
        .filter(uploads::filename.eq_any(candidates))
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)
}

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

pub(super) async fn resolve_upload_mime_type(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<String, HandlerError> {
    if let Ok(upload) = load_owned_upload(headers, filename, owner_profile_id).await {
        return Ok(upload.mime_type);
    }
    let has_owned_original = load_owned_original_for_variant(headers, filename, owner_profile_id)
        .await?
        .is_some();
    if !has_owned_original {
        return Err(Box::new(not_found(headers)));
    }
    Ok("image/webp".to_string())
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

pub(super) async fn build_signed_upload_redirect(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<Response, HandlerError> {
    let url = storage_signed_url(headers, filename).await?;
    Ok(Json(UploadUrlResponse { url }).into_response())
}
