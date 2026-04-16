use super::{
    bad_request, create_upload_filename, enqueue_upload_variants_generation, fallback_variant_urls,
    internal_upload_error, load_owned_upload, not_found, public_upload_url, require_auth_profile,
    storage_delete, storage_signed_put_url, storage_upload, uploads_multipart, uploads_resize,
    uploads_storage, validate_completed_upload_bytes, validate_filename, validate_presign_payload,
    AppContext, DataResponse, DirectUploadCompleteBody, DirectUploadPresignBody,
    DirectUploadPresignResponse, HandlerError, HeaderMap, Json, Multipart, NewUpload, Path,
    Response, Result, State, SuccessResponse, UploadChangeset, UploadResponse, Utc, Uuid,
};
use axum::response::IntoResponse;

use super::uploads_write_repo::{insert_upload_metadata, mark_upload_deleted};

const DIRECT_UPLOAD_PRESIGN_EXPIRY_SECS: u64 = 3600;

pub(in crate::api) async fn file_upload(
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

        if let Err(error) = insert_upload_metadata(&new_upload).await {
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
                id: upload_id.to_string(),
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

pub(in crate::api) async fn file_upload_presign(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DirectUploadPresignBody>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let context = validate_presign_payload(&headers, &payload)?;
        let profile = require_auth_profile(&headers).await?;

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

        insert_upload_metadata(&new_upload)
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

pub(in crate::api) async fn file_upload_complete(
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

        let bytes =
            uploads_storage::read(&payload.filename)
                .await
                .map_err(|error| match error.kind {
                    Some(uploads_storage::StorageErrorKind::NotFound) => {
                        Box::new(not_found(&headers)) as HandlerError
                    }
                    _ => internal_upload_error(&headers, "Upload storage is unavailable"),
                })?;
        validate_completed_upload_bytes(&headers, &upload.mime_type, &bytes)?;

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
                id: upload.id.to_string(),
                url,
                filename: upload.filename,
                size: bytes.len(),
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

pub(in crate::api) async fn file_delete(
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

        let changeset = UploadChangeset {
            deleted: Some(true),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        if let Err(error) = mark_upload_deleted(upload.id, &changeset).await {
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
