use axum::http::HeaderMap;
use base64::Engine;

fn dev_upload_url(filename: &str) -> String {
    format!("/api/v1/uploads/{filename}")
}

pub(super) async fn public_upload_url(headers: &HeaderMap, filename: &str) -> String {
    // Match resolve_image_url: serve the full/2000px WebP variant so
    // profile + event cover images stay sharp on high-DPI phones.
    if let Some(url) = crate::api::imgproxy_signing::signed_url(filename, "full", "webp") {
        return url;
    }
    let _ = headers;
    dev_upload_url(filename)
}

pub(super) async fn fallback_variant_urls(
    headers: &HeaderMap,
    original_filename: &str,
) -> (Option<String>, Option<String>) {
    if crate::api::imgproxy_signing::is_configured() {
        // Upload response's thumbnail_url + standard_url. thumbnail stays
        // at 200px for avatar-sized renders; standard is the full 2000px
        // WebP so swipe cards / detail views look sharp.
        let thumb = crate::api::imgproxy_signing::signed_avatar_url(original_filename);
        let full = crate::api::imgproxy_signing::signed_url(original_filename, "full", "webp");
        return (thumb, full);
    }
    let fallback = public_upload_url(headers, original_filename).await;
    (Some(fallback.clone()), Some(fallback))
}

pub(super) fn encode_thumbhash(raw: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(raw)
}
