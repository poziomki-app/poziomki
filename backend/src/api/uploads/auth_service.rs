use crate::db::models::profiles::Profile;
use crate::db::models::uploads::Upload;
use axum::http::HeaderMap;
use diesel_async::AsyncPgConnection;

use super::uploads_http::{forbidden, not_found};
use super::uploads_multipart::HandlerError;
use super::uploads_read_repo::{
    find_active_upload_by_filename, find_owned_active_upload_by_filenames, load_profile_by_user_id,
};
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

/// Load the caller's profile inside an existing viewer-scoped transaction.
/// A missing profile row is a 403 `ACCESS_DENIED` ("Profile not found");
/// a Diesel-level failure (connection drop, timeout, etc.) surfaces as a
/// 500 `INTERNAL_ERROR` so real DB problems don't get masked as a client
/// permission issue.
pub(super) async fn load_profile_for_user(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<Profile, HandlerError> {
    match load_profile_by_user_id(conn, user_id).await {
        Ok(Some(profile)) => Ok(profile),
        Ok(None) => Err(Box::new(forbidden(
            headers,
            "ACCESS_DENIED",
            "Profile not found",
        ))),
        Err(_) => Err(internal_upload_error(headers, "Failed to load profile")),
    }
}

pub(super) async fn load_owned_upload(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<Upload, HandlerError> {
    let upload = find_active_upload_by_filename(conn, filename)
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
    conn: &mut AsyncPgConnection,
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

    find_owned_active_upload_by_filenames(conn, owner_profile_id, &candidates)
        .await
        .map_err(|_| Box::new(not_found(headers)) as HandlerError)
}

pub(super) async fn resolve_upload_mime_type(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    filename: &str,
    owner_profile_id: uuid::Uuid,
) -> std::result::Result<String, HandlerError> {
    if let Ok(upload) = load_owned_upload(conn, headers, filename, owner_profile_id).await {
        return Ok(upload.mime_type);
    }
    let has_owned_original =
        load_owned_original_for_variant(conn, headers, filename, owner_profile_id)
            .await?
            .is_some();
    if !has_owned_original {
        return Err(Box::new(not_found(headers)));
    }
    Ok("image/webp".to_string())
}
