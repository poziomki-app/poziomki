use opendal::{
    services::{Fs, S3},
    ErrorKind, Operator,
};
use std::{sync::OnceLock, time::Duration};
use url::Url;

fn is_production_mode() -> bool {
    std::env::var("NODE_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

const DEFAULT_UPLOADS_DIR: &str = "../data/uploads";
const DEFAULT_REGION: &str = "us-east-1";
const DEFAULT_PRESIGN_EXPIRY_SECS: u64 = 3600;

#[derive(Clone)]
enum StorageMode {
    Production,
    Development,
}

#[derive(Clone)]
struct StorageConfig {
    operator: Operator,
    mode: StorageMode,
    public_url: Option<String>,
    presign_expiry_secs: u64,
}

static STORAGE: OnceLock<Result<StorageConfig, String>> = OnceLock::new();

#[derive(Clone)]
pub(super) struct StorageError {
    pub(super) kind: Option<ErrorKind>,
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

fn build_s3_operator() -> Result<StorageConfig, String> {
    let endpoint = env_any(&["GARAGE_S3_ENDPOINT"])
        .ok_or_else(|| "Missing S3 endpoint. Set GARAGE_S3_ENDPOINT.".to_string())?;
    let bucket = env_any(&["GARAGE_S3_BUCKET"])
        .ok_or_else(|| "Missing S3 bucket. Set GARAGE_S3_BUCKET.".to_string())?;
    let access_key = env_any(&["GARAGE_S3_ACCESS_KEY"])
        .ok_or_else(|| "Missing S3 access key. Set GARAGE_S3_ACCESS_KEY.".to_string())?;
    let secret_key = env_any(&["GARAGE_S3_SECRET_KEY"])
        .ok_or_else(|| "Missing S3 secret key. Set GARAGE_S3_SECRET_KEY.".to_string())?;
    let region = env_any(&["GARAGE_S3_REGION"]).unwrap_or_else(|| DEFAULT_REGION.to_string());
    let public_url = env_any(&["GARAGE_S3_PUBLIC_URL"]);
    let virtual_host = parse_bool_env("GARAGE_S3_VIRTUAL_HOST_STYLE", false);

    let builder = {
        let mut base = S3::default()
            .root("/")
            .bucket(&bucket)
            .endpoint(&endpoint)
            .region(&region)
            .access_key_id(&access_key)
            .secret_access_key(&secret_key)
            .disable_config_load();
        if virtual_host {
            base = base.enable_virtual_host_style();
        }
        base
    };

    let operator = Operator::new(builder)
        .map_err(|err| format!("Failed to build S3 operator: {err}"))?
        .finish();

    Ok(StorageConfig {
        operator,
        mode: StorageMode::Production,
        public_url,
        presign_expiry_secs: parse_presign_expiry_secs(),
    })
}

fn build_fs_operator() -> Result<StorageConfig, String> {
    let root = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| DEFAULT_UPLOADS_DIR.to_string());
    std::fs::create_dir_all(&root)
        .map_err(|err| format!("Failed to create uploads directory '{root}': {err}"))?;

    let operator = Operator::new(Fs::default().root(&root))
        .map_err(|err| format!("Failed to build filesystem operator: {err}"))?
        .finish();

    Ok(StorageConfig {
        operator,
        mode: StorageMode::Development,
        public_url: None,
        presign_expiry_secs: parse_presign_expiry_secs(),
    })
}

fn load_storage() -> Result<StorageConfig, String> {
    if is_production_mode() {
        build_s3_operator()
    } else {
        build_fs_operator()
    }
}

fn storage() -> Result<&'static StorageConfig, String> {
    let result = STORAGE.get_or_init(load_storage);
    result.as_ref().map_err(Clone::clone)
}

fn rewrite_signed_url(url: &str, public_url: &str) -> Option<String> {
    let mut signed = Url::parse(url).ok()?;
    let public = Url::parse(public_url).ok()?;
    signed.set_scheme(public.scheme()).ok()?;
    signed.set_host(public.host_str()).ok()?;
    if signed.set_port(public.port()).is_err() {
        return None;
    }
    Some(signed.to_string())
}

pub(super) async fn upload(
    filename: &str,
    bytes: &[u8],
    mime_type: &str,
) -> Result<(), StorageError> {
    let config = storage().map_err(|_message| StorageError { kind: None })?;
    config
        .operator
        .write_with(filename, bytes.to_vec())
        .content_type(mime_type)
        .await
        .map(|_| ())
        .map_err(|err| StorageError {
            kind: Some(err.kind()),
        })
}

pub(super) async fn exists(filename: &str) -> Result<bool, StorageError> {
    let config = storage().map_err(|_message| StorageError { kind: None })?;
    config
        .operator
        .exists(filename)
        .await
        .map_err(|err| StorageError {
            kind: Some(err.kind()),
        })
}

pub(super) async fn read(filename: &str) -> Result<Vec<u8>, StorageError> {
    let config = storage().map_err(|_message| StorageError { kind: None })?;
    config
        .operator
        .read(filename)
        .await
        .map(|buffer| buffer.to_vec())
        .map_err(|err| StorageError {
            kind: Some(err.kind()),
        })
}

pub(super) async fn delete(filename: &str) -> Result<(), StorageError> {
    let config = storage().map_err(|_message| StorageError { kind: None })?;
    config
        .operator
        .delete(filename)
        .await
        .map_err(|err| StorageError {
            kind: Some(err.kind()),
        })
}

pub(super) async fn signed_get_url(filename: &str) -> Result<String, StorageError> {
    let config = storage().map_err(|_message| StorageError { kind: None })?;
    match config.mode {
        StorageMode::Development => Ok(format!("/api/v1/uploads/{filename}")),
        StorageMode::Production => {
            let signed = config
                .operator
                .presign_read(filename, Duration::from_secs(config.presign_expiry_secs))
                .await
                .map_err(|err| StorageError {
                    kind: Some(err.kind()),
                })?
                .uri()
                .to_string();
            if let Some(public_url) = &config.public_url {
                return Ok(rewrite_signed_url(&signed, public_url).unwrap_or(signed));
            }
            Ok(signed)
        }
    }
}
