#[path = "uploads_multipart.rs"]
mod uploads_multipart;
#[path = "uploads_resize.rs"]
mod uploads_resize;
#[path = "uploads_storage.rs"]
pub(super) mod uploads_storage;
#[path = "uploads_support.rs"]
mod uploads_support;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use base64::Engine;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::{
    allowed_upload_mime, create_upload_filename, is_chat_context, is_production_mode,
    max_upload_size_bytes, parse_upload_context, require_auth_db, validate_filename, DataResponse,
    DirectUploadCompleteBody, DirectUploadPresignBody, DirectUploadPresignResponse,
    SuccessResponse, UploadResponse, UploadStatusResponse, UploadUrlResponse,
};
use super::{error_response, ErrorSpec};
use crate::db::models::profiles::Profile;
use crate::db::models::uploads::{NewUpload, Upload, UploadChangeset};
use crate::db::schema::{profiles, uploads};
use crate::tasks::enqueue_upload_variants_generation;
use uploads_multipart::HandlerError;
use uploads_support::{
    bad_request, forbidden, not_found, storage_delete, storage_read, storage_signed_put_url,
    storage_signed_url, storage_upload,
};

pub(super) struct AuthCheckResponse {
    pub(super) ok: bool,
}

impl serde::Serialize for AuthCheckResponse {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AuthCheckResponse", 1)?;
        state.serialize_field("ok", &self.ok)?;
        state.end()
    }
}

#[allow(clippy::unnecessary_box_returns)]
fn internal_upload_error(headers: &HeaderMap, message: &str) -> HandlerError {
    Box::new(error_response(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "INTERNAL_ERROR",
            details: None,
        },
    ))
}

fn dev_upload_url(filename: &str) -> String {
    format!("/api/v1/uploads/{filename}")
}

async fn public_upload_url(headers: &HeaderMap, filename: &str) -> String {
    if is_production_mode() {
        storage_signed_url(headers, filename)
            .await
            .unwrap_or_else(|_| dev_upload_url(filename))
    } else {
        dev_upload_url(filename)
    }
}

async fn fallback_variant_urls(
    headers: &HeaderMap,
    original_filename: &str,
) -> (Option<String>, Option<String>) {
    let fallback = public_upload_url(headers, original_filename).await;
    (Some(fallback.clone()), Some(fallback))
}

fn encode_thumbhash(raw: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(raw)
}

const DIRECT_UPLOAD_PRESIGN_EXPIRY_SECS: u64 = 3600;

async fn require_auth_profile(headers: &HeaderMap) -> std::result::Result<Profile, HandlerError> {
    let (_session, user) = require_auth_db(headers)
        .await
        .map_err(|e| e as HandlerError)?;

    let mut conn = crate::db::conn().await.map_err(|_| {
        Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
    })?;

    profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|_| {
            Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
        })?
        .ok_or_else(|| Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")))
}

async fn load_owned_upload(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Upload, HandlerError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?;

    let upload = uploads::table
        .filter(uploads::filename.eq(filename))
        .filter(uploads::deleted.eq(false))
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?
        .ok_or_else(|| Box::new(not_found(headers)) as HandlerError)?;

    if upload.owner_id != Some(owner_profile_id) {
        return Err(Box::new(forbidden(
            headers,
            "ACCESS_DENIED",
            "File access denied",
        )));
    }

    Ok(upload)
}

fn variant_stem(filename: &str) -> Option<&str> {
    filename
        .strip_suffix("_thumb.webp")
        .or_else(|| filename.strip_suffix("_std.webp"))
}

async fn load_owned_original_for_variant(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Option<Upload>, HandlerError> {
    let Some(stem) = variant_stem(filename) else {
        return Ok(None);
    };

    let candidates = vec![
        format!("{stem}.jpg"),
        format!("{stem}.jpeg"),
        format!("{stem}.png"),
        format!("{stem}.webp"),
        format!("{stem}.avif"),
    ];

    let mut conn = crate::db::conn()
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)?;

    uploads::table
        .filter(uploads::owner_id.eq(Some(owner_profile_id)))
        .filter(uploads::deleted.eq(false))
        .filter(uploads::filename.eq_any(candidates))
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)
}

fn extract_filename_from_original_uri(
    headers: &HeaderMap,
) -> std::result::Result<String, HandlerError> {
    let original_uri = headers
        .get("x-original-uri")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            Box::new(bad_request(
                headers,
                "MISSING_URI",
                "Missing X-Original-URI header",
            )) as HandlerError
        })?;

    let path = original_uri.split('?').next().unwrap_or_default();
    let filename = path
        .strip_prefix("/uploads/")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            Box::new(bad_request(headers, "INVALID_URI", "Invalid upload URI")) as HandlerError
        })?;

    if let Err(message) = validate_filename(filename) {
        return Err(Box::new(bad_request(headers, "INVALID_FILENAME", message)));
    }

    Ok(filename.to_string())
}

pub(super) async fn auth_check(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let profile = require_auth_profile(&headers).await?;
        let filename = extract_filename_from_original_uri(&headers)?;
        match load_owned_upload(&headers, &filename, profile.id).await {
            Ok(_upload) => {}
            Err(owned_err) => {
                let has_owned_original =
                    load_owned_original_for_variant(&headers, &filename, profile.id)
                        .await?
                        .is_some();
                if !has_owned_original {
                    return Err(owned_err);
                }
            }
        }

        Ok(Json(AuthCheckResponse { ok: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&headers).await?;
        let mime_type = if let Ok(upload) = load_owned_upload(&headers, &filename, profile.id).await
        {
            upload.mime_type
        } else {
            let has_owned_original =
                load_owned_original_for_variant(&headers, &filename, profile.id)
                    .await?
                    .is_some();
            if !has_owned_original {
                return Err(Box::new(not_found(&headers)));
            }
            "image/webp".to_string()
        };

        if is_production_mode() {
            let url = storage_signed_url(&headers, &filename).await?;
            Ok(Json(UploadUrlResponse { url }).into_response())
        } else {
            let bytes = storage_read(&headers, &filename).await?;
            let mut response = bytes.into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(&mime_type)
                    .unwrap_or(HeaderValue::from_static("application/octet-stream")),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=31536000"),
            );
            Ok(response)
        }
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_upload(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let profile = require_auth_profile(&headers).await?;

        let parsed = uploads_multipart::read_multipart(&headers, multipart).await?;
        let filename = create_upload_filename(&parsed.mime_type);
        storage_upload(&headers, &filename, &parsed.bytes, &parsed.mime_type).await?;
        let (thumbnail_url, standard_url) = fallback_variant_urls(&headers, &filename).await;

        let context_str = format!("{:?}", parsed.context).to_ascii_lowercase();
        let now = Utc::now();
        let upload_id = Uuid::new_v4();
        let new_upload = NewUpload {
            id: upload_id,
            filename: filename.clone(),
            owner_id: Some(profile.id),
            context: context_str,
            context_id: parsed.context_id,
            mime_type: parsed.mime_type.clone(),
            deleted: false,
            thumbhash: None,
            has_variants: false,
            created_at: now,
            updated_at: now,
        };

        let mut conn = crate::db::conn()
            .await
            .map_err(|_| internal_upload_error(&headers, "Failed to save upload metadata"))?;

        if let Err(error) = diesel::insert_into(uploads::table)
            .values(&new_upload)
            .execute(&mut conn)
            .await
        {
            tracing::warn!(filename = %filename, %error, "failed to insert upload row");
            let _ = uploads_storage::delete(&filename).await;
            return Err(internal_upload_error(
                &headers,
                "Failed to save upload metadata",
            ));
        }

        if let Err(error) = enqueue_upload_variants_generation(&upload_id).await {
            tracing::warn!(
                %error,
                upload_id = %upload_id,
                filename = %filename,
                "failed to enqueue upload variants generation"
            );
        }

        let url = public_upload_url(&headers, &filename).await;

        Ok(Json(DataResponse {
            data: UploadResponse {
                url,
                filename,
                size: parsed.bytes.len(),
                mime_type: parsed.mime_type,
                thumbnail_url,
                standard_url,
                thumbhash: None,
                processing: Some(true),
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|resp| *resp))
}

pub(super) async fn file_upload_presign(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DirectUploadPresignBody>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if !is_production_mode() {
            return Err(Box::new(bad_request(
                &headers,
                "DIRECT_UPLOAD_UNAVAILABLE",
                "Direct upload presign is available only in production/Garage mode",
            )));
        }

        let profile = require_auth_profile(&headers).await?;
        let context = parse_upload_context(payload.context.as_deref()).ok_or_else(|| {
            Box::new(bad_request(
                &headers,
                "VALIDATION_ERROR",
                "Invalid upload context",
            )) as HandlerError
        })?;

        if is_chat_context(context) && payload.context_id.as_deref().is_none_or(str::is_empty) {
            return Err(Box::new(bad_request(
                &headers,
                "MISSING_CONTEXT_ID",
                "contextId required for chat uploads",
            )));
        }
        if !allowed_upload_mime(&payload.mime_type) {
            return Err(Box::new(bad_request(
                &headers,
                "INVALID_FILE_TYPE",
                "Allowed: image/jpeg, image/png, image/webp",
            )));
        }
        if payload.size == 0 || payload.size > max_upload_size_bytes() {
            return Err(Box::new(bad_request(
                &headers,
                "FILE_TOO_LARGE",
                "Max: 10MB",
            )));
        }

        let filename = create_upload_filename(&payload.mime_type);
        let upload_url = storage_signed_put_url(&headers, &filename, &payload.mime_type).await?;

        let context_str = format!("{context:?}").to_ascii_lowercase();
        let now = Utc::now();
        let new_upload = NewUpload {
            id: Uuid::new_v4(),
            filename: filename.clone(),
            owner_id: Some(profile.id),
            context: context_str,
            context_id: payload.context_id.clone().filter(|s| !s.trim().is_empty()),
            mime_type: payload.mime_type.clone(),
            deleted: false,
            thumbhash: None,
            has_variants: false,
            created_at: now,
            updated_at: now,
        };

        let mut conn = crate::db::conn()
            .await
            .map_err(|_| internal_upload_error(&headers, "Failed to save upload metadata"))?;

        diesel::insert_into(uploads::table)
            .values(&new_upload)
            .execute(&mut conn)
            .await
            .map_err(|error| {
                tracing::warn!(%error, filename = %filename, "failed to insert direct-upload metadata row");
                internal_upload_error(&headers, "Failed to save upload metadata")
            })?;

        Ok(Json(DataResponse {
            data: DirectUploadPresignResponse {
                upload_url,
                method: "PUT",
                filename,
                mime_type: payload.mime_type,
                expires_in: DIRECT_UPLOAD_PRESIGN_EXPIRY_SECS,
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_upload_complete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DirectUploadCompleteBody>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&payload.filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&headers).await?;
        let upload = load_owned_upload(&headers, &payload.filename, profile.id).await?;

        let exists = uploads_storage::exists(&payload.filename)
            .await
            .map_err(|_error| {
                Box::new(error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &headers,
                    ErrorSpec {
                        error: "Upload storage is unavailable".to_string(),
                        code: "INTERNAL_ERROR",
                        details: None,
                    },
                )) as HandlerError
            })?;
        if !exists {
            return Err(Box::new(not_found(&headers)));
        }

        if let Err(error) = enqueue_upload_variants_generation(&upload.id).await {
            tracing::warn!(
                %error,
                upload_id = %upload.id,
                filename = %upload.filename,
                "failed to enqueue upload variants generation after direct upload complete"
            );
        }

        let url = public_upload_url(&headers, &upload.filename).await;
        let (thumbnail_url, standard_url) = fallback_variant_urls(&headers, &upload.filename).await;

        Ok(Json(DataResponse {
            data: UploadResponse {
                url,
                filename: upload.filename,
                size: 0,
                mime_type: upload.mime_type,
                thumbnail_url,
                standard_url,
                thumbhash: None,
                processing: Some(true),
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&headers).await?;
        let upload = load_owned_upload(&headers, &filename, profile.id).await?;

        let url = public_upload_url(&headers, &upload.filename).await;
        let (thumbnail_url, standard_url) = if upload.has_variants {
            let thumb_name = uploads_resize::variant_filename(&upload.filename, "thumb");
            let std_name = uploads_resize::variant_filename(&upload.filename, "std");
            (
                Some(public_upload_url(&headers, &thumb_name).await),
                Some(public_upload_url(&headers, &std_name).await),
            )
        } else {
            fallback_variant_urls(&headers, &upload.filename).await
        };
        let thumbhash = upload.thumbhash.as_deref().map(encode_thumbhash);

        Ok(Json(DataResponse {
            data: UploadStatusResponse {
                filename: upload.filename,
                url,
                thumbnail_url,
                standard_url,
                thumbhash,
                processing: !upload.has_variants,
                has_variants: upload.has_variants,
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn generate_upload_variants_job(
    upload_id: Uuid,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|error| error.to_string())?;

    let Some(upload) = uploads::table
        .find(upload_id)
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };

    if upload.deleted || upload.has_variants {
        return Ok(());
    }

    let original_bytes = uploads_storage::read(&upload.filename)
        .await
        .map_err(|error| format!("read original upload failed: {error:?}"))?;

    let variants = uploads_resize::generate_variants(&original_bytes, &upload.mime_type).await?;
    let thumb_name = uploads_resize::variant_filename(&upload.filename, "thumb");
    let std_name = uploads_resize::variant_filename(&upload.filename, "std");

    let (thumb_upload, std_upload) = tokio::join!(
        uploads_storage::upload(&thumb_name, &variants.thumbnail, "image/webp"),
        uploads_storage::upload(&std_name, &variants.standard, "image/webp")
    );

    let thumb_ok = thumb_upload.is_ok();
    let std_ok = std_upload.is_ok();
    if !(thumb_ok && std_ok) {
        if thumb_ok {
            let _ = uploads_storage::delete(&thumb_name).await;
        }
        if std_ok {
            let _ = uploads_storage::delete(&std_name).await;
        }

        let thumb_err = thumb_upload.err().map(|e| format!("{e:?}"));
        let std_err = std_upload.err().map(|e| format!("{e:?}"));
        return Err(format!(
            "variant upload incomplete for {}: thumb_ok={} std_ok={} thumb_err={thumb_err:?} std_err={std_err:?}",
            upload.filename, thumb_ok, std_ok
        ));
    }

    let changeset = UploadChangeset {
        thumbhash: Some(Some(variants.thumbhash)),
        has_variants: Some(true),
        updated_at: Some(Utc::now()),
        ..Default::default()
    };

    diesel::update(uploads::table.find(upload.id))
        .set(&changeset)
        .execute(&mut conn)
        .await
        .map_err(|error| format!("update upload variants metadata failed: {error}"))?;

    Ok(())
}

pub(super) async fn file_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&headers).await?;
        let upload = load_owned_upload(&headers, &filename, profile.id).await?;

        storage_delete(&headers, &filename).await?;

        // Best-effort delete variants regardless of DB flags to avoid stale files.
        let thumb_name = uploads_resize::variant_filename(&filename, "thumb");
        let std_name = uploads_resize::variant_filename(&filename, "std");
        let _ = uploads_storage::delete(&thumb_name).await;
        let _ = uploads_storage::delete(&std_name).await;

        // Mark as deleted
        let changeset = UploadChangeset {
            deleted: Some(true),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        let mut conn = crate::db::conn()
            .await
            .map_err(|_| internal_upload_error(&headers, "Failed to update upload metadata"))?;

        if let Err(error) = diesel::update(uploads::table.find(upload.id))
            .set(&changeset)
            .execute(&mut conn)
            .await
        {
            tracing::warn!(filename = %filename, %error, "failed to mark upload as deleted");
            return Err(internal_upload_error(
                &headers,
                "Failed to update upload metadata",
            ));
        }

        Ok(Json(SuccessResponse { success: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|response| *response))
}
