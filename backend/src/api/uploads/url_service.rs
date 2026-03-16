use axum::http::HeaderMap;
use base64::Engine;

fn dev_upload_url(filename: &str) -> String {
    format!("/api/v1/uploads/{filename}")
}

pub(super) async fn public_upload_url(headers: &HeaderMap, filename: &str, format: &str) -> String {
    if let Some(url) = crate::api::imgproxy_signing::signed_url(filename, "feed", format) {
        return url;
    }
    let _ = headers;
    dev_upload_url(filename)
}

pub(super) async fn fallback_variant_urls(
    headers: &HeaderMap,
    original_filename: &str,
    format: &str,
) -> (Option<String>, Option<String>) {
    if crate::api::imgproxy_signing::is_configured() {
        let thumb = crate::api::imgproxy_signing::signed_avatar_url(original_filename, format);
        let feed = crate::api::imgproxy_signing::signed_url(original_filename, "feed", format);
        return (thumb, feed);
    }
    let fallback = public_upload_url(headers, original_filename, format).await;
    (Some(fallback.clone()), Some(fallback))
}

pub(super) fn encode_thumbhash(raw: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(raw)
}
