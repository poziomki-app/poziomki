use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine;

use super::uploads_http::storage_signed_url;
use super::uploads_multipart::HandlerError;
use crate::api::state::{is_s3_storage_configured, UploadUrlResponse};

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

pub(super) async fn build_signed_upload_redirect(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<Response, HandlerError> {
    let url = storage_signed_url(headers, filename).await?;
    Ok(Json(UploadUrlResponse { url }).into_response())
}
