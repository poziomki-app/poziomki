use super::{
    AppContext, DataResponse, HandlerError, HeaderMap, HeaderValue, Json, Path, Response, Result,
    State, UploadStatusResponse, bad_request, encode_thumbhash, extract_filename_from_original_uri,
    fallback_variant_urls, header, load_owned_original_for_variant, load_owned_upload,
    public_upload_url, require_auth_profile, resolve_upload_mime_type, storage_read,
    uploads_resize, validate_filename,
};
use axum::response::IntoResponse;

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

pub(in crate::api) async fn auth_check(
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

        let profile = require_auth_profile(&headers).await?;
        let mime_type = resolve_upload_mime_type(&headers, &filename, profile.id).await?;

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

        let profile = require_auth_profile(&headers).await?;
        let upload = load_owned_upload(&headers, &filename, profile.id).await?;

        let imgproxy = crate::api::imgproxy_signing::is_configured();
        let url = public_upload_url(&headers, &upload.filename).await;
        let (thumbnail_url, standard_url) = if imgproxy {
            (
                crate::api::imgproxy_signing::signed_url(&upload.filename, "thumb", "webp"),
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
