use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

/// Authenticate via `require_auth_db` and early-return on failure.
macro_rules! auth_or_respond {
    ($headers:expr) => {
        match $crate::api::state::require_auth_db(&$headers).await {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        }
    };
}

pub(crate) use auth_or_respond;

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

/// Render an email for log output as `abc***@domain`, keeping the domain
/// (useful when triaging deployment-specific issues) while dropping most of
/// the local-part. Malformed addresses collapse to `***` so nothing sneaks
/// through in raw form.
pub fn redact_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "***".to_string();
    };
    let prefix: String = local.chars().take(3).collect();
    if prefix.is_empty() {
        return format!("***@{domain}");
    }
    format!("{prefix}***@{domain}")
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

pub fn parse_uuid(id: &str, label: &str) -> std::result::Result<Uuid, crate::error::AppError> {
    Uuid::parse_str(id).map_err(|_| crate::error::AppError::Message(format!("Invalid {label} ID")))
}

pub fn parse_uuid_response(
    id: &str,
    label: &str,
    headers: &HeaderMap,
) -> std::result::Result<Uuid, Box<Response>> {
    Uuid::parse_str(id).map_err(|_| {
        Box::new(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: format!("Invalid {label} ID"),
                code: "BAD_REQUEST",
                details: None,
            },
        ))
    })
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
/// Prefers imgproxy (full/2000px, webp) when configured; otherwise uses app-routed media URL.
///
/// 2000px WebP looks sharp on a 1080px high-DPI phone screen at any
/// rendered size up to full-bleed. 800px (the prior default) was
/// visibly soft on matching cards and bio galleries. Bandwidth cost
/// is ~3× per image but a WebP-compressed 2000px photo is still
/// ~200-400 KiB — acceptable for a beta audience on wifi.
pub async fn resolve_image_url(stored: &str) -> String {
    let filename = extract_filename(stored);
    if let Some(url) = super::imgproxy_signing::signed_url(&filename, "full", "webp") {
        return url;
    }
    format!("/api/v1/uploads/{filename}")
}

/// Resolve multiple image URLs in parallel.
pub async fn resolve_image_urls(stored: &[String]) -> Vec<String> {
    let futs: Vec<_> = stored.iter().map(|s| resolve_image_url(s)).collect();
    futures_util::future::join_all(futs).await
}

/// Resolve image URLs embedded in bio markdown `![](url)` patterns.
/// Replaces each URL (signed or plain filename) with a fresh signed URL.
#[allow(clippy::string_slice)] // markers are ASCII; byte indices from find() are char boundaries
pub async fn resolve_bio_image_urls(bio: &str) -> String {
    let mut result = String::with_capacity(bio.len());
    let mut search_from = 0;
    let marker = "![](";

    while let Some(start) = bio.get(search_from..).and_then(|s| s.find(marker)) {
        let abs_start = search_from + start;
        let url_start = abs_start + marker.len();
        let Some(rel_end) = bio.get(url_start..).and_then(|s| s.find(')')) else {
            break;
        };
        let url_end = url_start + rel_end;
        // All markers are ASCII so byte indices are valid char boundaries
        debug_assert!(bio.is_char_boundary(search_from));
        debug_assert!(bio.is_char_boundary(url_start));
        debug_assert!(bio.is_char_boundary(url_end));
        let inner_url = &bio[url_start..url_end];
        let resolved = resolve_image_url(inner_url).await;
        result.push_str(&bio[search_from..url_start]);
        result.push_str(&resolved);
        result.push(')');
        search_from = url_end + 1;
    }
    result.push_str(&bio[search_from..]);
    result
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

#[cfg(test)]
mod tests {
    use super::redact_email;

    #[test]
    fn keeps_three_char_prefix_and_domain() {
        assert_eq!(redact_email("adam@ibims.pl"), "ada***@ibims.pl");
    }

    #[test]
    fn handles_short_local_part() {
        assert_eq!(redact_email("ab@example.com"), "ab***@example.com");
    }

    #[test]
    fn collapses_malformed_address() {
        assert_eq!(redact_email("no-at-sign"), "***");
    }

    #[test]
    fn handles_empty_local_part() {
        assert_eq!(redact_email("@example.com"), "***@example.com");
    }
}
