#[path = "uploads_multipart.rs"]
mod uploads_multipart;
#[path = "uploads_resize.rs"]
mod uploads_resize;
#[path = "uploads_storage.rs"]
pub(super) mod uploads_storage;
#[path = "uploads_support.rs"]
mod uploads_support;

use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, ColumnTrait, QueryFilter};
use uuid::Uuid;

use super::state::{
    create_upload_filename, is_production_mode, require_auth_db, validate_filename, DataResponse,
    SuccessResponse, UploadResponse, UploadUrlResponse,
};
use crate::models::_entities::{profiles, uploads};
use uploads_multipart::HandlerError;
use uploads_support::{
    bad_request, forbidden, not_found, storage_delete, storage_read, storage_signed_url,
    storage_upload,
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

async fn require_auth_profile(
    db: &DatabaseConnection,
    headers: &HeaderMap,
) -> std::result::Result<profiles::Model, HandlerError> {
    let (_session, user) = require_auth_db(db, headers)
        .await
        .map_err(|e| e as HandlerError)?;
    profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(db)
        .await
        .map_err(|_| {
            Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
        })?
        .ok_or_else(|| Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")))
}

async fn load_owned_upload(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<uploads::Model, HandlerError> {
    let upload = uploads::Entity::find()
        .filter(uploads::Column::Filename.eq(filename))
        .filter(uploads::Column::Deleted.eq(false))
        .one(db)
        .await
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
    db: &DatabaseConnection,
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Option<uploads::Model>, HandlerError> {
    let Some(stem) = variant_stem(filename) else {
        return Ok(None);
    };

    let candidates = [
        format!("{stem}.jpg"),
        format!("{stem}.jpeg"),
        format!("{stem}.png"),
        format!("{stem}.webp"),
        format!("{stem}.avif"),
    ];

    uploads::Entity::find()
        .filter(uploads::Column::OwnerId.eq(Some(owner_profile_id)))
        .filter(uploads::Column::Deleted.eq(false))
        .filter(uploads::Column::Filename.is_in(candidates))
        .one(db)
        .await
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
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let profile = require_auth_profile(&ctx.db, &headers).await?;
        let filename = extract_filename_from_original_uri(&headers)?;
        let _upload = load_owned_upload(&ctx.db, &headers, &filename, profile.id).await?;

        Ok(Json(AuthCheckResponse { ok: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_get(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&ctx.db, &headers).await?;
        let mime_type =
            if let Ok(upload) = load_owned_upload(&ctx.db, &headers, &filename, profile.id).await {
                upload.mime_type
            } else {
                let has_owned_original =
                    load_owned_original_for_variant(&ctx.db, &headers, &filename, profile.id)
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
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let profile = require_auth_profile(&ctx.db, &headers).await?;

        let parsed = uploads_multipart::read_multipart(&headers, multipart).await?;
        let filename = create_upload_filename(&parsed.mime_type);
        storage_upload(&headers, &filename, &parsed.bytes, &parsed.mime_type).await?;

        // Generate image variants (thumbnail + standard WebP)
        let variants = uploads_resize::generate_variants(&parsed.bytes, &parsed.mime_type).await;

        let mut thumbhash_bytes: Option<Vec<u8>> = None;
        let mut has_variants = false;
        let mut thumbnail_url: Option<String> = None;
        let mut standard_url: Option<String> = None;
        let mut thumbhash_b64: Option<String> = None;

        match variants {
            Ok(v) => {
                let thumb_name = uploads_resize::variant_filename(&filename, "thumb");
                let std_name = uploads_resize::variant_filename(&filename, "std");

                let (thumb_upload, std_upload) = tokio::join!(
                    storage_upload(&headers, &thumb_name, &v.thumbnail, "image/webp"),
                    storage_upload(&headers, &std_name, &v.standard, "image/webp")
                );

                let thumb_ok = thumb_upload.is_ok();
                let std_ok = std_upload.is_ok();

                has_variants = thumb_ok && std_ok;
                thumbhash_bytes = Some(v.thumbhash.clone());
                thumbhash_b64 = Some(uploads_resize::encode_thumbhash_base64(&v.thumbhash));

                if has_variants {
                    thumbnail_url = if is_production_mode() {
                        storage_signed_url(&headers, &thumb_name).await.ok()
                    } else {
                        Some(format!("/api/v1/uploads/{thumb_name}"))
                    };
                    standard_url = if is_production_mode() {
                        storage_signed_url(&headers, &std_name).await.ok()
                    } else {
                        Some(format!("/api/v1/uploads/{std_name}"))
                    };
                } else {
                    if thumb_ok {
                        let _ = uploads_storage::delete(&thumb_name).await;
                    }
                    if std_ok {
                        let _ = uploads_storage::delete(&std_name).await;
                    }
                    tracing::warn!(
                        filename = %filename,
                        thumb_ok,
                        std_ok,
                        "variant upload incomplete; cleaned partial variant files"
                    );
                }
            }
            Err(err) => {
                tracing::warn!(filename = %filename, error = %err, "image variant generation failed");
            }
        }

        // Store upload record in DB
        let context_str = format!("{:?}", parsed.context).to_ascii_lowercase();
        let now = Utc::now();
        let upload = uploads::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            filename: ActiveValue::Set(filename.clone()),
            owner_id: ActiveValue::Set(Some(profile.id)),
            context: ActiveValue::Set(context_str),
            context_id: ActiveValue::Set(parsed.context_id),
            mime_type: ActiveValue::Set(parsed.mime_type.clone()),
            deleted: ActiveValue::Set(false),
            thumbhash: ActiveValue::Set(thumbhash_bytes),
            has_variants: ActiveValue::Set(has_variants),
            created_at: ActiveValue::Set(now.into()),
            updated_at: ActiveValue::Set(now.into()),
        };
        let _ = upload.insert(&ctx.db).await;

        let url = if is_production_mode() {
            storage_signed_url(&headers, &filename).await?
        } else {
            format!("/api/v1/uploads/{filename}")
        };

        Ok(Json(DataResponse {
            data: UploadResponse {
                url,
                filename,
                size: parsed.bytes.len(),
                mime_type: parsed.mime_type,
                thumbnail_url,
                standard_url,
                thumbhash: thumbhash_b64,
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|resp| *resp))
}

pub(super) async fn file_delete(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let profile = require_auth_profile(&ctx.db, &headers).await?;
        let upload = load_owned_upload(&ctx.db, &headers, &filename, profile.id).await?;

        storage_delete(&headers, &filename).await?;

        // Best-effort delete variants regardless of DB flags to avoid stale files.
        let thumb_name = uploads_resize::variant_filename(&filename, "thumb");
        let std_name = uploads_resize::variant_filename(&filename, "std");
        let _ = uploads_storage::delete(&thumb_name).await;
        let _ = uploads_storage::delete(&std_name).await;

        // Mark as deleted
        let mut active: uploads::ActiveModel = upload.into();
        active.deleted = ActiveValue::Set(true);
        active.updated_at = ActiveValue::Set(Utc::now().into());
        let _ = active.update(&ctx.db).await;

        Ok(Json(SuccessResponse { success: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|response| *response))
}
