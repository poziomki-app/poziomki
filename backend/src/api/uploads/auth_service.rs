use crate::db::models::profiles::Profile;
use crate::db::models::uploads::Upload;
use axum::http::HeaderMap;

use super::uploads_http::{forbidden, not_found};
use super::uploads_multipart::HandlerError;
use super::uploads_read_repo::{
    find_active_upload_by_filename, find_owned_active_upload_by_filenames, load_profile_by_user_id,
};
use crate::api::state::require_auth_db;
use crate::api::{error_response, ErrorSpec};

#[allow(clippy::unnecessary_box_returns)]
pub(super) fn internal_upload_error(headers: &HeaderMap, message: &str) -> HandlerError {
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

pub(super) async fn require_auth_profile(
    headers: &HeaderMap,
) -> std::result::Result<Profile, HandlerError> {
    let (_session, user) = require_auth_db(headers)
        .await
        .map_err(|e| e as HandlerError)?;

    load_profile_by_user_id(user.id)
        .await
        .map_err(|_| {
            Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")) as HandlerError
        })?
        .ok_or_else(|| Box::new(forbidden(headers, "ACCESS_DENIED", "Profile not found")))
}

pub(super) async fn load_owned_upload(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Upload, HandlerError> {
    let upload = find_active_upload_by_filename(filename)
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

pub(super) async fn load_owned_original_for_variant(
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
    ];

    find_owned_active_upload_by_filenames(owner_profile_id, &candidates)
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)
}

pub(super) async fn resolve_upload_mime_type(
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<String, HandlerError> {
    if let Ok(upload) = load_owned_upload(headers, filename, owner_profile_id).await {
        return Ok(upload.mime_type);
    }
    let has_owned_original = load_owned_original_for_variant(headers, filename, owner_profile_id)
        .await?
        .is_some();
    if !has_owned_original {
        return Err(Box::new(not_found(headers)));
    }
    Ok("image/webp".to_string())
}
