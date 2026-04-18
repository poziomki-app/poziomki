use super::{
    bad_request, encode_thumbhash, extract_filename_from_original_uri, fallback_variant_urls,
    header, load_owned_original_for_variant, load_owned_upload, load_profile_for_user,
    public_upload_url, resolve_upload_mime_type, storage_read, uploads_resize, validate_filename,
    AppContext, DataResponse, HandlerError, HeaderMap, HeaderValue, Json, Path, Response, Result,
    State, UploadStatusResponse,
};
use axum::response::IntoResponse;
use diesel_async::scoped_futures::ScopedFutureExt;

use crate::api::{error_response, ErrorSpec};
use crate::db;
use crate::db::models::uploads::Upload;

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

/// Translate the outcome of a viewer-scoped transaction that wraps
/// `HandlerError`-returning work into the handler's `Result<T, HandlerError>`
/// shape. `Ok(Ok(t))` means the tx committed and the business result is
/// `t`; `Ok(Err(e))` means the work produced a response-level error
/// (rolled back); `Err(db)` means Diesel itself failed.
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

pub(in crate::api) async fn auth_check(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let (_session, user) = crate::api::state::require_auth_db(&headers)
            .await
            .map_err(|e| e as HandlerError)?;
        let filename = extract_filename_from_original_uri(&headers)?;
        let viewer = db::DbViewer {
            user_id: user.id,
            is_review_stub: user.is_review_stub,
        };

        let headers_tx = headers.clone();
        let filename_tx = filename.clone();
        let tx_result = db::with_viewer_tx(viewer, move |conn| {
            async move {
                let profile = match load_profile_for_user(conn, &headers_tx, user.id).await {
                    Ok(p) => p,
                    Err(err) => return Ok(Err(err)),
                };
                match load_owned_upload(conn, &headers_tx, &filename_tx, profile.id).await {
                    Ok(_) => Ok::<_, diesel::result::Error>(Ok(())),
                    Err(owned_err) => {
                        let has_owned_original = match load_owned_original_for_variant(
                            conn,
                            &headers_tx,
                            &filename_tx,
                            profile.id,
                        )
                        .await
                        {
                            Ok(v) => v.is_some(),
                            Err(err) => return Ok(Err(err)),
                        };
                        if has_owned_original {
                            Ok(Ok(()))
                        } else {
                            Ok(Err(owned_err))
                        }
                    }
                }
            }
            .scope_boxed()
        })
        .await;

        unwrap_viewer_tx(tx_result, &headers)?;
        Ok(Json(AuthCheckResponse { ok: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

fn build_local_file_response(bytes: Vec<u8>, mime_type: &str) -> Response {
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000"),
    );
    response
}

pub(in crate::api) async fn file_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let (_session, user) = crate::api::state::require_auth_db(&headers)
            .await
            .map_err(|e| e as HandlerError)?;
        let viewer = db::DbViewer {
            user_id: user.id,
            is_review_stub: user.is_review_stub,
        };

        let headers_tx = headers.clone();
        let filename_tx = filename.clone();
        let tx_result = db::with_viewer_tx(viewer, move |conn| {
            async move {
                let profile = match load_profile_for_user(conn, &headers_tx, user.id).await {
                    Ok(p) => p,
                    Err(err) => return Ok(Err(err)),
                };
                match resolve_upload_mime_type(conn, &headers_tx, &filename_tx, profile.id).await {
                    Ok(mt) => Ok::<_, diesel::result::Error>(Ok(mt)),
                    Err(err) => Ok(Err(err)),
                }
            }
            .scope_boxed()
        })
        .await;

        let mime_type = unwrap_viewer_tx(tx_result, &headers)?;

        if let Some(url) = crate::api::imgproxy_signing::signed_url(&filename, "full", "webp") {
            return Ok(Json(super::super::state::UploadUrlResponse { url }).into_response());
        }
        let bytes = storage_read(&headers, &filename).await?;
        Ok(build_local_file_response(bytes, &mime_type))
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(in crate::api) async fn file_status(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        let (_session, user) = crate::api::state::require_auth_db(&headers)
            .await
            .map_err(|e| e as HandlerError)?;
        let viewer = db::DbViewer {
            user_id: user.id,
            is_review_stub: user.is_review_stub,
        };

        let headers_tx = headers.clone();
        let filename_tx = filename.clone();
        let tx_result = db::with_viewer_tx(viewer, move |conn| {
            async move {
                let profile = match load_profile_for_user(conn, &headers_tx, user.id).await {
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

        let upload: Upload = unwrap_viewer_tx(tx_result, &headers)?;

        let imgproxy = crate::api::imgproxy_signing::is_configured();
        let url = public_upload_url(&headers, &upload.filename).await;
        let (thumbnail_url, standard_url) = if imgproxy {
            (
                crate::api::imgproxy_signing::signed_avatar_url(&upload.filename),
                crate::api::imgproxy_signing::signed_url(&upload.filename, "feed", "webp"),
            )
        } else if upload.has_variants {
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
