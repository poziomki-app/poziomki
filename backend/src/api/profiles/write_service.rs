use axum::http::HeaderMap;
use axum::response::Response;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::{
    error_response, extract_filename,
    state::{
        require_auth_db, validate_filename, validate_profile_bio, validate_profile_name,
        validate_profile_program, CreateProfileBody, UpdateProfileBody,
    },
    ErrorSpec,
};
use crate::db::models::profiles::{Profile, ProfileChangeset};
use crate::db::models::uploads::Upload;
use crate::db::schema::{profiles, uploads};

// super = profiles_write_handler, super::super = profiles
use super::super::{not_found_profile, validation_error};

fn check_validation(
    headers: &HeaderMap,
    result: Result<(), &str>,
) -> std::result::Result<(), Box<Response>> {
    result.map_err(|msg| Box::new(validation_error(headers, msg)))
}

pub(super) fn validate_profile_fields(
    headers: &HeaderMap,
    payload: &CreateProfileBody,
) -> std::result::Result<(), Box<Response>> {
    check_validation(headers, validate_profile_name(&payload.name))?;
    check_validation(headers, validate_profile_bio(payload.bio.as_ref()))?;
    check_validation(headers, validate_profile_program(payload.program.as_ref()))
}

async fn check_no_existing_profile(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<(), Box<Response>> {
    let existing = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(conn)
        .await
        .optional()
        .map_err(|e| {
            tracing::error!(error = %e, "database error checking existing profile");
            Box::new(error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal server error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            ))
        })?;
    if existing.is_some() {
        return Err(Box::new(error_response(
            axum::http::StatusCode::CONFLICT,
            headers,
            ErrorSpec {
                error: "Profile already exists".to_string(),
                code: "CONFLICT",
                details: None,
            },
        )));
    }
    Ok(())
}

pub(super) async fn validate_create(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    payload: &CreateProfileBody,
) -> std::result::Result<crate::db::models::users::User, Box<Response>> {
    let (_session, user) = require_auth_db(headers).await?;
    validate_profile_fields(headers, payload)?;
    // Reject image references on create: uploads require an existing
    // profile (uploads/write_handler.rs:198) so a not-yet-created
    // profile cannot legitimately own any. Allowing arbitrary
    // filenames here lets a fresh account claim another user's
    // uploads as their profile picture or gallery. The canonical flow
    // is create-with-no-images → upload (now owner_id = new profile)
    // → PATCH images.
    if payload.profile_picture.is_some() {
        return Err(Box::new(validation_error(
            headers,
            "Profile picture cannot be set on create — upload it after the profile exists",
        )));
    }
    if payload.images.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(Box::new(validation_error(
            headers,
            "Gallery images cannot be set on create — upload them after the profile exists",
        )));
    }
    check_no_existing_profile(conn, headers, user.id).await?;
    Ok(user)
}

#[allow(clippy::unnecessary_box_returns)]
fn uploads_unavailable(headers: &HeaderMap) -> Box<Response> {
    Box::new(error_response(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        headers,
        ErrorSpec {
            error: "Upload storage is temporarily unavailable".to_string(),
            code: "UPLOADS_UNAVAILABLE",
            details: None,
        },
    ))
}

async fn verify_upload_ownership(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    profile_id: Uuid,
    filename: &str,
) -> std::result::Result<(), Box<Response>> {
    let owned_upload = uploads::table
        .filter(uploads::owner_id.eq(Some(profile_id)))
        .filter(uploads::filename.eq(filename))
        .filter(uploads::deleted.eq(false))
        .first::<Upload>(conn)
        .await
        .optional()
        .map_err(|_| uploads_unavailable(headers))?;

    if owned_upload.is_none() {
        return Err(Box::new(validation_error(
            headers,
            "Profile picture must reference your uploaded image",
        )));
    }
    Ok(())
}

/// Verify that every entry in `filenames` is a non-deleted upload owned
/// by `profile_id`. Without this, a caller could PATCH their profile
/// with another user's filename and the API would issue signed URLs
/// pointing at the original owner's bytes — content theft, no copy.
pub(super) async fn verify_uploads_ownership(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    profile_id: Uuid,
    filenames: &[String],
) -> std::result::Result<(), Box<Response>> {
    if filenames.is_empty() {
        return Ok(());
    }
    let owned: std::collections::HashSet<String> = uploads::table
        .filter(uploads::owner_id.eq(Some(profile_id)))
        .filter(uploads::filename.eq_any(filenames))
        .filter(uploads::deleted.eq(false))
        .select(uploads::filename)
        .load::<String>(conn)
        .await
        .map_err(|_| uploads_unavailable(headers))?
        .into_iter()
        .collect();
    // Compare set membership, not lengths: `eq_any` -> SQL IN dedupes,
    // so `["a.jpg", "a.jpg"]` would yield owned.len() = 1 even when
    // a.jpg is owned, and a length comparison would falsely reject.
    if filenames.iter().any(|f| !owned.contains(f)) {
        return Err(Box::new(validation_error(
            headers,
            "Profile images must reference your uploaded files",
        )));
    }
    Ok(())
}

async fn verify_upload_exists(
    headers: &HeaderMap,
    filename: &str,
) -> std::result::Result<(), Box<Response>> {
    let exists = crate::api::uploads::uploads_storage::exists(filename)
        .await
        .map_err(|_| uploads_unavailable(headers))?;
    if !exists {
        return Err(Box::new(validation_error(
            headers,
            "Profile picture file was not found in upload storage",
        )));
    }
    Ok(())
}

async fn validate_profile_picture_reference(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    owner_profile_id: Option<Uuid>,
    raw_picture: &str,
) -> std::result::Result<String, Box<Response>> {
    let filename = extract_filename(raw_picture);
    check_validation(headers, validate_filename(&filename))?;

    if let Some(profile_id) = owner_profile_id {
        verify_upload_ownership(conn, headers, profile_id, &filename).await?;
    }
    verify_upload_exists(headers, &filename).await?;

    Ok(filename)
}

pub(super) async fn resolve_picture_filename(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    owner_profile_id: Option<Uuid>,
    raw_picture: Option<&str>,
) -> std::result::Result<Option<String>, Box<Response>> {
    match raw_picture {
        Some(raw) => validate_profile_picture_reference(conn, headers, owner_profile_id, raw)
            .await
            .map(Some),
        None => Ok(None),
    }
}

#[allow(clippy::option_option)]
pub(super) async fn validate_and_prepare_update(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    id: &str,
    payload: &UpdateProfileBody,
) -> std::result::Result<
    (
        Profile,
        crate::db::models::users::User,
        Option<Option<String>>,
    ),
    Box<Response>,
> {
    let (profile, user) = load_and_verify_profile(conn, headers, id).await?;
    validate_update_payload(headers, payload)?;
    let picture = match &payload.profile_picture {
        None => None,             // absent → don't change
        Some(None) => Some(None), // explicit null → clear
        Some(Some(raw)) => {
            let filename =
                resolve_picture_filename(conn, headers, Some(profile.id), Some(raw.as_str()))
                    .await?;
            Some(filename) // Some(Some(filename))
        }
    };
    // Gallery images get the same ownership check as profile_picture —
    // every filename must reference an upload that this profile owns.
    if let Some(images) = payload.images.as_ref() {
        let filenames: Vec<String> = images.iter().map(|raw| extract_filename(raw)).collect();
        for f in &filenames {
            check_validation(headers, validate_filename(f))?;
        }
        verify_uploads_ownership(conn, headers, profile.id, &filenames).await?;
    }
    Ok((profile, user, picture))
}

fn validate_update_payload(
    headers: &HeaderMap,
    payload: &UpdateProfileBody,
) -> std::result::Result<(), Box<Response>> {
    if let Some(name) = &payload.name {
        check_validation(headers, validate_profile_name(name))?;
    }
    check_validation(headers, validate_profile_bio(payload.bio.as_ref()))?;
    check_validation(headers, validate_profile_program(payload.program.as_ref()))
}

fn non_empty_or_null(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn build_images_json(images: &[String]) -> Option<serde_json::Value> {
    let filenames: Vec<String> = images.iter().map(|s| extract_filename(s)).collect();
    serde_json::to_value(filenames).ok()
}

#[allow(clippy::option_option)]
pub(super) fn build_update_changeset(
    payload: &UpdateProfileBody,
    profile_picture: Option<Option<String>>,
) -> ProfileChangeset {
    ProfileChangeset {
        name: payload.name.as_ref().map(|n| n.trim().to_string()),
        bio: payload.bio.as_ref().map(|b| Some(b.clone())),
        program: payload.program.as_ref().map(|p| Some(p.clone())),
        profile_picture,
        images: payload.images.as_ref().map(|imgs| build_images_json(imgs)),
        gradient_start: payload.gradient_start.as_deref().map(non_empty_or_null),
        gradient_end: payload.gradient_end.as_deref().map(non_empty_or_null),
        updated_at: Some(chrono::Utc::now()),
    }
}

pub(super) async fn load_and_verify_profile(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Profile, crate::db::models::users::User), Box<Response>> {
    let (_session, user) = require_auth_db(headers).await?;
    let profile_uuid = crate::api::parse_uuid_response(id, "profile", headers)?;

    let profile = profiles::table
        .find(profile_uuid)
        .first::<Profile>(conn)
        .await
        .optional()
        .map_err(|_| Box::new(not_found_profile(headers, id)))?
        .ok_or_else(|| Box::new(not_found_profile(headers, id)))?;

    if profile.user_id != user.id {
        return Err(Box::new(error_response(
            axum::http::StatusCode::FORBIDDEN,
            headers,
            ErrorSpec {
                error: "You can only access your own profile".to_string(),
                code: "FORBIDDEN",
                details: None,
            },
        )));
    }

    Ok((profile, user))
}
