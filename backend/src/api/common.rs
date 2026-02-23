use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: &'static str,
    #[serde(rename = "requestId")]
    request_id: String,
    details: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct ErrorSpec {
    pub(crate) error: String,
    pub(crate) code: &'static str,
    pub(crate) details: Option<serde_json::Value>,
}

pub fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), ToOwned::to_owned)
}

pub fn error_response(
    status: axum::http::StatusCode,
    headers: &HeaderMap,
    spec: ErrorSpec,
) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: spec.error,
            code: spec.code,
            request_id: request_id(headers),
            details: spec.details,
        }),
    )
        .into_response()
}

/// Strip a presigned URL down to just the filename (last path segment).
/// If the value is already a plain filename, return it unchanged.
pub fn extract_filename(value: &str) -> String {
    if value.starts_with("http") {
        url::Url::parse(value)
            .ok()
            .and_then(|u| u.path_segments()?.next_back().map(ToString::to_string))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| value.to_string())
    } else {
        value.to_string()
    }
}

/// Resolve a stored image value (filename or legacy presigned URL) to a fresh signed URL.
/// Prefers imgproxy (feed/800px, webp) when configured; falls back to S3 presigned.
pub async fn resolve_image_url(stored: &str) -> String {
    let filename = extract_filename(stored);
    if let Some(url) = super::imgproxy_signing::signed_url(&filename, "feed", "webp") {
        return url;
    }
    super::uploads::uploads_storage::signed_get_url(&filename)
        .await
        .unwrap_or(filename)
}

/// Resolve multiple image URLs in parallel.
pub async fn resolve_image_urls(stored: &[String]) -> Vec<String> {
    let futs: Vec<_> = stored.iter().map(|s| resolve_image_url(s)).collect();
    futures_util::future::join_all(futs).await
}
