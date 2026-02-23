#[path = "http_read.rs"]
mod uploads_http_read;
#[path = "http_support.rs"]
mod uploads_http_support;
#[path = "http_write.rs"]
mod uploads_http_write;
#[path = "multipart.rs"]
mod uploads_multipart;
#[path = "resize.rs"]
mod uploads_resize;
#[path = "service.rs"]
mod uploads_service;
#[path = "storage.rs"]
pub(super) mod uploads_storage;
#[path = "variant_jobs.rs"]
mod uploads_variant_jobs;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::{
    create_upload_filename, is_s3_storage_configured, validate_filename, DataResponse,
    DirectUploadCompleteBody, DirectUploadPresignBody, DirectUploadPresignResponse,
    SuccessResponse, UploadResponse, UploadStatusResponse,
};
use super::{error_response, ErrorSpec};
use crate::db::models::uploads::{NewUpload, UploadChangeset};
use crate::db::schema::uploads;
use crate::jobs::enqueue_upload_variants_generation;
pub(super) use uploads_http_read::{auth_check, file_get, file_status};
use uploads_http_support::{
    bad_request, not_found, storage_delete, storage_read, storage_signed_put_url, storage_upload,
};
pub(super) use uploads_http_write::{
    file_delete, file_upload, file_upload_complete, file_upload_presign,
};
use uploads_multipart::HandlerError;
use uploads_service::{
    build_signed_upload_redirect, encode_thumbhash, extract_filename_from_original_uri,
    fallback_variant_urls, internal_upload_error, load_owned_original_for_variant,
    load_owned_upload, public_upload_url, require_auth_profile, resolve_upload_mime_type,
    validate_presign_payload,
};

pub(super) async fn generate_upload_variants_job(
    upload_id: Uuid,
) -> std::result::Result<(), String> {
    uploads_variant_jobs::generate_upload_variants_job(upload_id).await
}
