use axum::{
    Json,
    http::HeaderMap,
    response::{IntoResponse, Response},
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

/// Normalize an S3 object prefix: strip leading `/`, ensure trailing `/`,
/// reject path-traversal patterns.
pub fn normalize_object_prefix(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("object prefix must be non-empty".to_string());
    }
    if trimmed.contains("..") || trimmed.contains('\\') || trimmed.contains('\0') {
        return Err("object prefix contains invalid characters".to_string());
    }
    let mut prefix = trimmed.trim_start_matches('/').to_string();
    if !prefix.ends_with('/') {
        prefix.push('/');
    }
    Ok(prefix)
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
/// Prefers imgproxy (feed/800px, webp) when configured; otherwise uses app-routed media URL.
pub async fn resolve_image_url(stored: &str) -> String {
    let filename = extract_filename(stored);
    if let Some(url) = super::imgproxy_signing::signed_url(&filename, "feed", "webp") {
        return url;
    }
    format!("/api/v1/uploads/{filename}")
}

/// Resolve multiple image URLs in parallel.
pub async fn resolve_image_urls(stored: &[String]) -> Vec<String> {
    let futs: Vec<_> = stored.iter().map(|s| resolve_image_url(s)).collect();
    futures_util::future::join_all(futs).await
}

/// Look up thumbhashes for a batch of filenames.
/// Returns a map from filename → base64-encoded thumbhash.
pub async fn resolve_thumbhashes(
    filenames: &[String],
) -> std::collections::HashMap<String, String> {
    use crate::db::schema::uploads;
    use base64::Engine;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    if filenames.is_empty() {
        return std::collections::HashMap::new();
    }

    let rows: Vec<(String, Vec<u8>)> = match crate::db::conn().await {
        Ok(mut conn) => uploads::table
            .filter(uploads::filename.eq_any(filenames))
            .filter(uploads::thumbhash.is_not_null())
            .select((uploads::filename, uploads::thumbhash.assume_not_null()))
            .load::<(String, Vec<u8>)>(&mut conn)
            .await
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    rows.into_iter()
        .map(|(name, raw)| (name, base64::engine::general_purpose::STANDARD.encode(&raw)))
        .collect()
}
