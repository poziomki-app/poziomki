#[path = "auth_service.rs"]
mod uploads_auth_service;
#[path = "http.rs"]
mod uploads_http;
#[path = "multipart.rs"]
mod uploads_multipart;
#[path = "read_handler.rs"]
mod uploads_read_handler;
#[path = "read_repo.rs"]
mod uploads_read_repo;
#[path = "resize.rs"]
mod uploads_resize;
#[path = "storage.rs"]
pub(super) mod uploads_storage;
#[path = "url_service.rs"]
mod uploads_url_service;
#[path = "validation_service.rs"]
mod uploads_validation_service;
#[path = "variant_jobs.rs"]
mod uploads_variant_jobs;
#[path = "write_handler.rs"]
mod uploads_write_handler;
#[path = "write_repo.rs"]
mod uploads_write_repo;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use super::state::{
    create_upload_filename, validate_filename, DataResponse, DirectUploadCompleteBody,
    DirectUploadPresignBody, DirectUploadPresignResponse, SuccessResponse, UploadResponse,
    UploadStatusResponse,
};
use super::{error_response, ErrorSpec};
use crate::db::models::uploads::{NewUpload, UploadChangeset};
use crate::jobs::enqueue_upload_variants_generation;
use uploads_auth_service::{
    internal_upload_error, load_owned_original_for_variant, load_owned_upload,
    load_profile_for_user, resolve_upload_mime_type,
};
use uploads_http::{bad_request, not_found, storage_read, storage_signed_put_url, storage_upload};
use uploads_multipart::HandlerError;
pub(super) use uploads_read_handler::{auth_check, file_get, file_status};
use uploads_url_service::{encode_thumbhash, fallback_variant_urls, public_upload_url};
use uploads_validation_service::{extract_filename_from_original_uri, validate_presign_payload};
pub(super) use uploads_write_handler::{
    file_delete, file_upload, file_upload_complete, file_upload_presign,
};

pub(super) async fn generate_upload_variants_job(
    upload_id: Uuid,
) -> std::result::Result<(), String> {
    uploads_variant_jobs::generate_upload_variants_job(upload_id).await
}

pub(in crate::api) async fn read_upload_bytes(
    filename: &str,
) -> std::result::Result<Vec<u8>, crate::error::AppError> {
    uploads_storage::read(filename)
        .await
        .map_err(|_err| crate::error::AppError::message("failed to read upload from storage"))
}

/// Best-effort delete of an upload's S3 objects (original + thumb + std variants).
pub(in crate::api) async fn delete_upload_objects(filename: &str) {
    let _ = uploads_storage::delete(filename).await;
    let thumb = uploads_resize::variant_filename(filename, "thumb");
    let std = uploads_resize::variant_filename(filename, "std");
    let _ = uploads_storage::delete(&thumb).await;
    let _ = uploads_storage::delete(&std).await;
}
