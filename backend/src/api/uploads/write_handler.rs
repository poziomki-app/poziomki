use super::{
    bad_request, create_upload_filename, enqueue_upload_variants_generation, error_response,
    fallback_variant_urls, internal_upload_error, load_owned_upload, load_profile_for_user,
    public_upload_url, storage_head_meta, storage_read, storage_signed_put_url, storage_upload,
    uploads_multipart, uploads_resize, uploads_storage, validate_filename,
    validate_presign_payload, AppContext, DataResponse, DirectUploadCompleteBody,
    DirectUploadPresignBody, DirectUploadPresignResponse, ErrorSpec, HandlerError, HeaderMap, Json,
    Multipart, NewUpload, Path, Response, Result, State, SuccessResponse, UploadChangeset,
    UploadResponse, Utc, Uuid,
};
use crate::api::state::{
    allowed_upload_mime, max_upload_size_bytes, strip_image_metadata, validate_image_dimensions,
    validate_magic_bytes,
};
use axum::response::IntoResponse;
use diesel_async::scoped_futures::ScopedFutureExt;

use super::uploads_write_repo::{insert_upload_metadata, mark_upload_deleted};
use crate::db;

const DIRECT_UPLOAD_PRESIGN_EXPIRY_SECS: u64 = 3600;

/// Map a viewer-tx result carrying an inner `HandlerError` into the handler's
/// `Result<T, HandlerError>` shape. Diesel-level failures become a generic
/// 500 — matches the prior `?` behaviour which would have surfaced as
/// `AppError::Any`.
fn unwrap_viewer_tx<T>(
    result: std::result::Result<std::result::Result<T, HandlerError>, diesel::result::Error>,
    headers: &HeaderMap,
) -> std::result::Result<T, HandlerError> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err),
        Err(_) => Err(Box::new(error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Database error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ))),
    }
}

/// Best-effort cleanup after a rejected direct upload: drop the object
/// in storage and soft-delete the DB row. Runs on the "caller shipped
/// garbage" path so the next presign attempt isn't blocked by a stale
/// upload row + we don't retain the bytes.
async fn purge_rejected_upload(upload_id: Uuid, filename: &str) {
    let _ = uploads_storage::delete(filename).await;
    let changeset = UploadChangeset {
        deleted: Some(true),
        updated_at: Some(Utc::now()),
        ..Default::default()
    };
    if let Ok(mut conn) = crate::db::conn().await {
        let _ =
            super::uploads_write_repo::mark_upload_deleted(&mut conn, upload_id, &changeset).await;
    }
}

async fn sanitize_direct_upload(
    headers: &HeaderMap,
    upload: &crate::db::models::uploads::Upload,
    filename: &str,
) -> std::result::Result<(), HandlerError> {
    use axum::http::StatusCode;

    // 1. HEAD first. Cheaper than a full GET and lets us reject
    //    oversized / wrong-type uploads before pulling bytes over
    //    the wire.
    // A missing object surfaces from storage_head_meta as NOT_FOUND
    // (same shape the old exists() check returned).
    let meta = storage_head_meta(headers, filename).await?;

    let max = max_upload_size_bytes() as u64;
    if let Some(len) = meta.content_length {
        if len > max {
            purge_rejected_upload(upload.id, filename).await;
            return Err(Box::new(bad_request(
                headers,
                "FILE_TOO_LARGE",
                "Uploaded file exceeds maximum size",
            )));
        }
    }
    if let Some(ct) = meta.content_type.as_deref() {
        let normalized = ct
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !normalized.is_empty() && normalized != upload.mime_type.to_ascii_lowercase() {
            purge_rejected_upload(upload.id, filename).await;
            return Err(Box::new(bad_request(
                headers,
                "INVALID_FILE_CONTENT",
                "Uploaded content type does not match presign",
            )));
        }
    }
    if !allowed_upload_mime(&upload.mime_type) {
        purge_rejected_upload(upload.id, filename).await;
        return Err(Box::new(bad_request(
            headers,
            "INVALID_FILE_TYPE",
            "Unsupported file type",
        )));
    }

    // 2. Download bytes. Bounded by the HEAD check above.
    let bytes = storage_read(headers, filename).await.inspect_err(|_| {
        tracing::warn!(filename, "failed to read direct-upload bytes for sanitize");
    })?;
    if bytes.len() as u64 > max {
        purge_rejected_upload(upload.id, filename).await;
        return Err(Box::new(bad_request(
            headers,
            "FILE_TOO_LARGE",
            "Uploaded file exceeds maximum size",
        )));
    }

    // 3. Magic bytes + dimensions + strip-and-reencode. Same chain the
    //    multipart path runs.
    if !validate_magic_bytes(&bytes, &upload.mime_type) {
        purge_rejected_upload(upload.id, filename).await;
        return Err(Box::new(bad_request(
            headers,
            "INVALID_FILE_CONTENT",
            "Content does not match type",
        )));
    }
    if let Err(msg) = validate_image_dimensions(&bytes, &upload.mime_type) {
        purge_rejected_upload(upload.id, filename).await;
        return Err(Box::new(bad_request(headers, "IMAGE_TOO_LARGE", msg)));
    }
    let sanitized = match strip_image_metadata(&bytes, &upload.mime_type) {
        Ok(out) => out,
        Err(msg) => {
            purge_rejected_upload(upload.id, filename).await;
            return Err(Box::new(bad_request(headers, "INVALID_FILE_CONTENT", msg)));
        }
    };

    // 4. NSFW gate on the sanitized payload. Runs before the storage
    //    overwrite so a rejected image never lands in the canonical
    //    bucket, only the presigner's raw upload does — and that gets
    //    purged below.
    if let Err(rejection) =
        super::uploads_image_moderation::moderate_upload_image_or_reject(headers, &sanitized).await
    {
        purge_rejected_upload(upload.id, filename).await;
        return Err(rejection);
    }

    // 5. Overwrite with sanitized bytes so downstream readers +
    //    variant generation see metadata-stripped content.
    if let Err(err) = uploads_storage::upload(filename, &sanitized, &upload.mime_type).await {
        tracing::warn!(filename, ?err.kind, "failed to overwrite sanitized direct upload");
        return Err(Box::new(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Failed to finalize upload".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        )));
    }
    Ok(())
}

async fn auth_and_viewer(
    headers: &HeaderMap,
) -> std::result::Result<(db::DbViewer, i32), HandlerError> {
    let (_session, user) = crate::api::state::require_auth_db(headers)
        .await
        .map_err(|e| e as HandlerError)?;
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    Ok((viewer, user.id))
}

pub(in crate::api) async fn file_upload(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        crate::api::ip_rate_limit::enforce_ip_rate_limit(
            &headers,
            crate::api::ip_rate_limit::IpRateLimitAction::UploadWrite,
        )
        .await?;

        let (viewer, user_id) = auth_and_viewer(&headers).await?;

        // Resolve the caller's profile inside a short viewer tx before
        // touching storage, so we don't S3-upload bytes for a user who
        // has no profile yet.
        let headers_for_profile = headers.clone();
        let profile_tx = db::with_viewer_tx(viewer, move |conn| {
            async move {
                match load_profile_for_user(conn, &headers_for_profile, user_id).await {
                    Ok(p) => Ok::<_, diesel::result::Error>(Ok(p)),
                    Err(err) => Ok(Err(err)),
                }
            }
            .scope_boxed()
        })
        .await;
        let profile = unwrap_viewer_tx(profile_tx, &headers)?;

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

        // Insert metadata in its own viewer tx so Tier-C upload write policy
        // (owner_id = viewer's profile) is enforced once it lands.
        let insert_tx = db::with_viewer_tx(viewer, {
            let new_upload = new_upload.clone();
            move |conn| {
                async move {
                    insert_upload_metadata(conn, &new_upload)
                        .await
                        .map_err(|_| diesel::result::Error::RollbackTransaction)?;
                    Ok::<(), diesel::result::Error>(())
                }
                .scope_boxed()
            }
        })
        .await;
        if let Err(error) = insert_tx {
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
        crate::api::ip_rate_limit::enforce_ip_rate_limit(
            &headers,
            crate::api::ip_rate_limit::IpRateLimitAction::UploadWrite,
        )
        .await?;

        let context = validate_presign_payload(&headers, &payload)?;
        let (viewer, user_id) = auth_and_viewer(&headers).await?;

        let headers_for_profile = headers.clone();
        let profile_tx = db::with_viewer_tx(viewer, move |conn| {
            async move {
                match load_profile_for_user(conn, &headers_for_profile, user_id).await {
                    Ok(p) => Ok::<_, diesel::result::Error>(Ok(p)),
                    Err(err) => Ok(Err(err)),
                }
            }
            .scope_boxed()
        })
        .await;
        let profile = unwrap_viewer_tx(profile_tx, &headers)?;

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

        let insert_tx = db::with_viewer_tx(viewer, {
            let new_upload = new_upload.clone();
            move |conn| {
                async move {
                    insert_upload_metadata(conn, &new_upload)
                        .await
                        .map_err(|_| diesel::result::Error::RollbackTransaction)?;
                    Ok::<(), diesel::result::Error>(())
                }
                .scope_boxed()
            }
        })
        .await;
        insert_tx.map_err(|error| {
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
        crate::api::ip_rate_limit::enforce_ip_rate_limit(
            &headers,
            crate::api::ip_rate_limit::IpRateLimitAction::UploadWrite,
        )
        .await?;

        if let Err(message) = validate_filename(&payload.filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let (viewer, user_id) = auth_and_viewer(&headers).await?;

        let headers_tx = headers.clone();
        let filename_tx = payload.filename.clone();
        let tx_result = db::with_viewer_tx(viewer, move |conn| {
            async move {
                let profile = match load_profile_for_user(conn, &headers_tx, user_id).await {
                    Ok(p) => p,
                    Err(err) => return Ok(Err(err)),
                };
                match load_owned_upload(conn, &headers_tx, &filename_tx, profile.id).await {
                    Ok(upload) => Ok::<_, diesel::result::Error>(Ok(upload)),
                    Err(err) => Ok(Err(err)),
                }
            }
            .scope_boxed()
        })
        .await;
        let upload = unwrap_viewer_tx(tx_result, &headers)?;

        // The client PUT directly to Garage via presigned URL without
        // going through our multipart validator — so we must re-run the
        // same checks here: HEAD the object, enforce size + mime match,
        // download, verify magic bytes + dimensions, strip metadata, and
        // overwrite with the sanitized bytes. Otherwise a caller can
        // upload arbitrary non-image content up to whatever size Garage
        // accepts and trigger downstream variant generation on it.
        sanitize_direct_upload(&headers, &upload, &payload.filename).await?;

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

pub(in crate::api) async fn file_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let (viewer, user_id) = auth_and_viewer(&headers).await?;

        // Mark the upload row deleted in the same transaction that loads
        // it, so ownership check and soft-delete are atomic. Storage
        // cleanup happens *after* commit — swapping the old order prevents
        // the "live row pointing at missing storage" race when the DB
        // update fails after a successful S3 delete. If a later storage
        // op fails, the row is already invisible to reads (deleted=true)
        // and operators can clean up orphans later.
        let headers_tx = headers.clone();
        let filename_tx = filename.clone();
        let changeset = UploadChangeset {
            deleted: Some(true),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };
        let tx_result = db::with_viewer_tx(viewer, move |conn| {
            async move {
                let profile = match load_profile_for_user(conn, &headers_tx, user_id).await {
                    Ok(p) => p,
                    Err(err) => return Ok(Err(err)),
                };
                let upload = match load_owned_upload(conn, &headers_tx, &filename_tx, profile.id)
                    .await
                {
                    Ok(u) => u,
                    Err(err) => return Ok(Err(err)),
                };
                if let Err(error) = mark_upload_deleted(conn, upload.id, &changeset).await {
                    tracing::warn!(filename = %filename_tx, %error, "failed to mark upload as deleted");
                    return Err(diesel::result::Error::RollbackTransaction);
                }
                Ok::<_, diesel::result::Error>(Ok(()))
            }
            .scope_boxed()
        })
        .await;
        unwrap_viewer_tx(tx_result, &headers)?;

        // Best-effort storage cleanup. Original + thumb + std variants are
        // all dropped without failing the request — the row is already
        // marked deleted, so a storage failure leaves an orphan object
        // rather than an inconsistent API surface.
        let _ = uploads_storage::delete(&filename).await;
        let thumb_name = uploads_resize::variant_filename(&filename, "thumb");
        let std_name = uploads_resize::variant_filename(&filename, "std");
        let _ = uploads_storage::delete(&thumb_name).await;
        let _ = uploads_storage::delete(&std_name).await;

        Ok(Json(SuccessResponse { success: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|response| *response))
}
