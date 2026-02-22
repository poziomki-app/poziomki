type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::controllers::migration_api::{
    error_response, extract_filename,
    state::{
        require_auth_db, validate_filename, validate_profile_age, validate_profile_bio,
        validate_profile_name, validate_profile_program, CreateProfileBody, DataResponse,
        SuccessResponse, UpdateProfileBody,
    },
    ErrorSpec,
};
use crate::db::models::profiles::{NewProfile, Profile, ProfileChangeset};
use crate::db::models::uploads::Upload;
use crate::db::schema::{profiles, uploads};
use crate::tasks::enqueue_matrix_profile_avatar_sync;

use super::{
    full_profile_response, not_found_profile, parse_tag_uuids, sync_profile_tags, validation_error,
};

fn validate_profile_fields(
    headers: &HeaderMap,
    payload: &CreateProfileBody,
) -> std::result::Result<(), Box<Response>> {
    if let Err(msg) = validate_profile_name(&payload.name) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    if let Err(msg) = validate_profile_age(payload.age) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    if let Err(msg) = validate_profile_bio(payload.bio.as_ref()) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    if let Err(msg) = validate_profile_program(payload.program.as_ref()) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    Ok(())
}

async fn check_no_existing_profile(
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<(), Box<Response>> {
    let mut conn = crate::db::conn().await.map_err(|e| {
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

    let existing = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(&mut conn)
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

async fn validate_create(
    headers: &HeaderMap,
    payload: &CreateProfileBody,
) -> std::result::Result<crate::db::models::users::User, Box<Response>> {
    let (_session, user) = require_auth_db(headers).await?;
    validate_profile_fields(headers, payload)?;
    check_no_existing_profile(headers, user.id).await?;
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

async fn validate_profile_picture_reference(
    headers: &HeaderMap,
    owner_profile_id: Option<Uuid>,
    raw_picture: &str,
) -> std::result::Result<String, Box<Response>> {
    let filename = extract_filename(raw_picture);
    if let Err(message) = validate_filename(&filename) {
        return Err(Box::new(validation_error(headers, message)));
    }

    if let Some(profile_id) = owner_profile_id {
        let mut conn = crate::db::conn()
            .await
            .map_err(|_| uploads_unavailable(headers))?;

        let owned_upload = uploads::table
            .filter(uploads::owner_id.eq(Some(profile_id)))
            .filter(uploads::filename.eq(&filename))
            .filter(uploads::deleted.eq(false))
            .first::<Upload>(&mut conn)
            .await
            .optional()
            .map_err(|_| uploads_unavailable(headers))?;

        if owned_upload.is_none() {
            return Err(Box::new(validation_error(
                headers,
                "Profile picture must reference your uploaded image",
            )));
        }
    }

    let exists = super::super::uploads::uploads_storage::exists(&filename)
        .await
        .map_err(|_| uploads_unavailable(headers))?;
    if !exists {
        return Err(Box::new(validation_error(
            headers,
            "Profile picture file was not found in upload storage",
        )));
    }

    Ok(filename)
}

fn build_create_model(
    user: &crate::db::models::users::User,
    payload: &CreateProfileBody,
    profile_picture: Option<String>,
) -> (NewProfile, Uuid) {
    let now = Utc::now();
    let profile_id = Uuid::new_v4();
    let images_json = payload.images.as_ref().and_then(|imgs| {
        serde_json::to_value(imgs.iter().map(|s| extract_filename(s)).collect::<Vec<_>>()).ok()
    });

    let model = NewProfile {
        id: profile_id,
        user_id: user.id,
        name: payload.name.trim().to_string(),
        bio: payload.bio.clone(),
        age: i16::from(payload.age),
        profile_picture,
        images: images_json,
        program: payload.program.clone(),
        gradient_start: payload.gradient_start.clone(),
        gradient_end: payload.gradient_end.clone(),
        created_at: now,
        updated_at: now,
    };
    (model, profile_id)
}

pub(in crate::controllers::migration_api) async fn profile_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateProfileBody>,
) -> Result<Response> {
    let user = match validate_create(&headers, &payload).await {
        Ok(u) => u,
        Err(response) => return Ok(*response),
    };
    let requested_profile_picture = match payload.profile_picture.as_deref() {
        Some(raw_picture) => {
            match validate_profile_picture_reference(&headers, None, raw_picture).await {
                Ok(filename) => Some(filename),
                Err(response) => return Ok(*response),
            }
        }
        None => None,
    };
    let should_sync_matrix_avatar = requested_profile_picture.is_some();

    let (new_profile, profile_id) = build_create_model(&user, &payload, requested_profile_picture);

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let inserted = diesel::insert_into(profiles::table)
        .values(&new_profile)
        .get_result::<Profile>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_ids = parse_tag_uuids(payload.tags.or(payload.tag_ids));
    if !tag_ids.is_empty() {
        sync_profile_tags(profile_id, &tag_ids).await?;
    }

    crate::search::invalidate_search_cache();

    if should_sync_matrix_avatar {
        let user_pid = user.pid;
        if let Err(error) =
            enqueue_matrix_profile_avatar_sync(&user_pid, inserted.profile_picture.as_deref()).await
        {
            tracing::warn!(%error, user_pid = %user_pid, "failed to enqueue matrix avatar sync after profile create");
        }
    }

    let data = full_profile_response(&inserted, &user.pid).await?;
    Ok((axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response())
}

fn validate_update_payload(
    headers: &HeaderMap,
    payload: &UpdateProfileBody,
) -> std::result::Result<(), Box<Response>> {
    if let Some(name) = &payload.name {
        if let Err(msg) = validate_profile_name(name) {
            return Err(Box::new(validation_error(headers, msg)));
        }
    }
    if let Some(age) = payload.age {
        if let Err(msg) = validate_profile_age(age) {
            return Err(Box::new(validation_error(headers, msg)));
        }
    }
    if let Err(msg) = validate_profile_bio(payload.bio.as_ref()) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    if let Err(msg) = validate_profile_program(payload.program.as_ref()) {
        return Err(Box::new(validation_error(headers, msg)));
    }
    Ok(())
}

fn build_update_changeset(
    payload: &UpdateProfileBody,
    profile_picture: Option<String>,
) -> ProfileChangeset {
    let mut changeset = ProfileChangeset::default();

    if let Some(name) = &payload.name {
        changeset.name = Some(name.trim().to_string());
    }
    if let Some(age) = payload.age {
        changeset.age = Some(i16::from(age));
    }
    if let Some(bio) = &payload.bio {
        changeset.bio = Some(Some(bio.clone()));
    }
    if let Some(program) = &payload.program {
        changeset.program = Some(Some(program.clone()));
    }
    if let Some(pic) = profile_picture {
        changeset.profile_picture = Some(Some(pic));
    }
    if let Some(images) = &payload.images {
        let filenames: Vec<String> = images.iter().map(|s| extract_filename(s)).collect();
        changeset.images = Some(serde_json::to_value(filenames).ok());
    }
    if let Some(gs) = &payload.gradient_start {
        changeset.gradient_start = Some(if gs.is_empty() {
            None
        } else {
            Some(gs.clone())
        });
    }
    if let Some(ge) = &payload.gradient_end {
        changeset.gradient_end = Some(if ge.is_empty() {
            None
        } else {
            Some(ge.clone())
        });
    }
    changeset.updated_at = Some(Utc::now());

    changeset
}

async fn load_and_verify_profile(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Profile, crate::db::models::users::User), Box<Response>> {
    let (_session, user) = require_auth_db(headers).await?;
    let profile_uuid = Uuid::parse_str(id).map_err(|_| {
        Box::new(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Invalid profile ID".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ))
    })?;

    let mut conn = crate::db::conn().await.map_err(|_| {
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

    let profile = profiles::table
        .find(profile_uuid)
        .first::<Profile>(&mut conn)
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

async fn maybe_sync_tags(
    profile_id: Uuid,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> std::result::Result<(), crate::error::AppError> {
    if tags.is_some() || tag_ids.is_some() {
        let resolved = parse_tag_uuids(tags.or(tag_ids));
        sync_profile_tags(profile_id, &resolved).await?;
    }
    Ok(())
}

pub(in crate::controllers::migration_api) async fn profile_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileBody>,
) -> Result<Response> {
    let (profile, user) = match load_and_verify_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };
    if let Err(response) = validate_update_payload(&headers, &payload) {
        return Ok(*response);
    }
    let requested_profile_picture = match payload.profile_picture.as_deref() {
        Some(raw_picture) => {
            match validate_profile_picture_reference(&headers, Some(profile.id), raw_picture).await
            {
                Ok(filename) => Some(filename),
                Err(response) => return Ok(*response),
            }
        }
        None => None,
    };
    let should_sync_matrix_avatar = requested_profile_picture.is_some();

    let profile_uuid = profile.id;
    let changeset = build_update_changeset(&payload, requested_profile_picture);

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let updated = diesel::update(profiles::table.find(profile_uuid))
        .set(&changeset)
        .get_result::<Profile>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    maybe_sync_tags(profile_uuid, payload.tags, payload.tag_ids).await?;

    crate::search::invalidate_search_cache();

    if should_sync_matrix_avatar {
        let user_pid = user.pid;
        if let Err(error) =
            enqueue_matrix_profile_avatar_sync(&user_pid, updated.profile_picture.as_deref()).await
        {
            tracing::warn!(%error, user_pid = %user_pid, "failed to enqueue matrix avatar sync after profile update");
        }
    }

    let data = full_profile_response(&updated, &user.pid).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::controllers::migration_api) async fn profile_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (profile, _user) = match load_and_verify_profile(&headers, &id).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    diesel::delete(profiles::table.find(profile.id))
        .execute(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    crate::search::invalidate_search_cache();

    Ok(Json(SuccessResponse { success: true }).into_response())
}
