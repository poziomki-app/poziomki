use super::{
    bad_request, create_upload_filename, enqueue_upload_variants_generation, error_response,
    fallback_variant_urls, internal_upload_error, load_owned_upload, load_profile_for_user,
    not_found, public_upload_url, storage_signed_put_url, storage_upload, uploads_multipart,
    uploads_resize, uploads_storage, validate_filename, validate_presign_payload, AppContext,
    DataResponse, DirectUploadCompleteBody, DirectUploadPresignBody, DirectUploadPresignResponse,
    ErrorSpec, HandlerError, HeaderMap, Json, Multipart, NewUpload, Path, Response, Result, State,
    SuccessResponse, UploadChangeset, UploadResponse, Utc, Uuid,
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
