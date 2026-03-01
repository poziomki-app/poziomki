use axum::http::{HeaderMap, HeaderValue, header};
use s3::{Bucket, Region, creds::Credentials, error::S3Error};
use std::sync::OnceLock;
use url::Url;

const DEFAULT_REGION: &str = "garage";
const DEFAULT_PRESIGN_EXPIRY_SECS: u64 = 3600;
const MAX_PRESIGN_EXPIRY_SECS: u32 = 604_800;

#[derive(Clone)]
struct StorageConfig {
    bucket: Box<Bucket>,
    public_url: Option<String>,
    presign_expiry_secs: u64,
    object_prefix: String,
}

static STORAGE: OnceLock<Result<StorageConfig, String>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::api) enum StorageErrorKind {
    NotFound,
}

#[derive(Clone, Debug)]
pub(in crate::api) struct StorageError {
    pub(in crate::api) kind: Option<StorageErrorKind>,
}

const fn storage_error(kind: Option<StorageErrorKind>) -> StorageError {
    StorageError { kind }
}

fn parse_bool_env(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|raw| match raw.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn env_any(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
}

fn parse_presign_expiry_secs() -> u64 {
    env_any(&["GARAGE_S3_URL_EXPIRY"])
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_PRESIGN_EXPIRY_SECS)
}

fn presign_expiry_secs_u32(seconds: u64) -> u32 {
    let bounded = seconds.min(u64::from(MAX_PRESIGN_EXPIRY_SECS));
    u32::try_from(bounded).unwrap_or(MAX_PRESIGN_EXPIRY_SECS)
}

fn normalize_prefix(raw: &str) -> Result<String, String> {
    crate::api::common::normalize_object_prefix(raw)
}

fn object_path(object_prefix: &str, filename: &str) -> String {
    format!("/{object_prefix}{filename}")
}

fn build_bucket() -> Result<StorageConfig, String> {
    let endpoint = env_any(&["GARAGE_S3_ENDPOINT"])
        .ok_or_else(|| "Missing S3 endpoint. Set GARAGE_S3_ENDPOINT.".to_string())?;
    let bucket_name = env_any(&["GARAGE_S3_BUCKET"])
        .ok_or_else(|| "Missing S3 bucket. Set GARAGE_S3_BUCKET.".to_string())?;
    let access_key = env_any(&["GARAGE_S3_ACCESS_KEY"])
        .ok_or_else(|| "Missing S3 access key. Set GARAGE_S3_ACCESS_KEY.".to_string())?;
    let secret_key = env_any(&["GARAGE_S3_SECRET_KEY"])
        .ok_or_else(|| "Missing S3 secret key. Set GARAGE_S3_SECRET_KEY.".to_string())?;
    let region_name = env_any(&["GARAGE_S3_REGION"]).unwrap_or_else(|| DEFAULT_REGION.to_string());
    let public_url = env_any(&["GARAGE_S3_PUBLIC_URL"]);
    let virtual_host = parse_bool_env("GARAGE_S3_VIRTUAL_HOST_STYLE", false);
    let object_prefix = normalize_prefix(
        env_any(&["IMGPROXY_ALLOWED_PREFIX"])
            .as_deref()
            .unwrap_or("uploads/"),
    )?;

    let region = Region::Custom {
        region: region_name,
        endpoint,
    };
    let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
        .map_err(|err| format!("Failed to build S3 credentials: {err}"))?;
    let bucket = Bucket::new(&bucket_name, region, credentials)
        .map_err(|err| format!("Failed to build S3 bucket client: {err}"))?;
    let bucket = if virtual_host {
        bucket
    } else {
        bucket.with_path_style()
    };

    Ok(StorageConfig {
        bucket,
        public_url,
        presign_expiry_secs: parse_presign_expiry_secs(),
        object_prefix,
    })
}

fn load_storage() -> Result<StorageConfig, String> {
    build_bucket()
}

fn storage() -> Result<&'static StorageConfig, String> {
    let result = STORAGE.get_or_init(load_storage);
    result.as_ref().map_err(Clone::clone)
}

fn try_rewrite_signed_url(url: &str, public_url: &str) -> Option<String> {
    let mut signed = Url::parse(url).ok()?;
    let public = Url::parse(public_url).ok()?;
    signed.set_scheme(public.scheme()).ok()?;
    signed.set_host(public.host_str()).ok()?;
    if signed.set_port(public.port()).is_err() {
        return None;
    }
    Some(signed.to_string())
}

fn rewrite_signed_url(url: &str, public_url: &str) -> String {
    try_rewrite_signed_url(url, public_url).unwrap_or_else(|| {
        tracing::warn!(
            url,
            public_url,
            "failed to rewrite presigned URL to public URL"
        );
        url.to_string()
    })
}

const fn map_s3_error(err: &S3Error) -> StorageError {
    match err {
        S3Error::HttpFailWithBody(404, _) => storage_error(Some(StorageErrorKind::NotFound)),
        _ => storage_error(None),
    }
}

fn ensure_ok_status(status_code: u16) -> Result<(), StorageError> {
    if (200..300).contains(&status_code) {
        Ok(())
    } else if status_code == 404 {
        Err(storage_error(Some(StorageErrorKind::NotFound)))
    } else {
        Err(storage_error(None))
    }
}

pub(super) async fn upload(
    filename: &str,
    bytes: &[u8],
    mime_type: &str,
) -> Result<(), StorageError> {
    let config = storage().map_err(|_message| storage_error(None))?;
    let response = config
        .bucket
        .put_object_with_content_type(
            object_path(&config.object_prefix, filename),
            bytes,
            mime_type,
        )
        .await
        .map_err(|err| map_s3_error(&err))?;
    ensure_ok_status(response.status_code())
}

pub(in crate::api) async fn read(filename: &str) -> Result<Vec<u8>, StorageError> {
    let config = storage().map_err(|_message| storage_error(None))?;
    let response = config
        .bucket
        .get_object(object_path(&config.object_prefix, filename))
        .await
        .map_err(|err| map_s3_error(&err))?;
    ensure_ok_status(response.status_code())?;
    Ok(response.to_vec())
}

pub(in crate::api) async fn exists(filename: &str) -> Result<bool, StorageError> {
    let config = storage().map_err(|_message| storage_error(None))?;
    let (_head, status_code) = config
        .bucket
        .head_object(object_path(&config.object_prefix, filename))
        .await
        .map_err(|err| map_s3_error(&err))?;
    if (200..300).contains(&status_code) {
        return Ok(true);
    }
    if status_code == 404 {
        return Ok(false);
    }
    Err(storage_error(None))
}

pub(super) async fn delete(filename: &str) -> Result<(), StorageError> {
    let config = storage().map_err(|_message| storage_error(None))?;
    let response = config
        .bucket
        .delete_object(object_path(&config.object_prefix, filename))
        .await
        .map_err(|err| map_s3_error(&err))?;
    ensure_ok_status(response.status_code())
}

pub(in crate::api) async fn signed_put_url(
    filename: &str,
    mime_type: &str,
) -> Result<String, StorageError> {
    let config = storage().map_err(|_message| storage_error(None))?;
    let mut headers = HeaderMap::new();
    let content_type = HeaderValue::from_str(mime_type).map_err(|_err| storage_error(None))?;
    headers.insert(header::CONTENT_TYPE, content_type);
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000"),
    );

    let signed = config
        .bucket
        .presign_put(
            object_path(&config.object_prefix, filename),
            presign_expiry_secs_u32(config.presign_expiry_secs),
            Some(headers),
            None,
        )
        .await
        .map_err(|err| map_s3_error(&err))?;
    if let Some(public_url) = &config.public_url {
        return Ok(rewrite_signed_url(&signed, public_url));
    }
    Ok(signed)
}
