use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};
use sea_orm::{ActiveValue, ColumnTrait, QueryFilter};
use uuid::Uuid;

type Result<T> = crate::error::AppResult<T>;

use crate::controllers::migration_api::{
    error_response, extract_filename,
    state::{
        require_auth_db, validate_filename, validate_profile_age, validate_profile_bio,
        validate_profile_name, validate_profile_program, CreateProfileBody, DataResponse,
        SuccessResponse, UpdateProfileBody,
    },
    ErrorSpec,
};
use crate::models::_entities::{profiles, uploads};
use crate::tasks::enqueue_matrix_profile_avatar_sync;

use super::{
    full_profile_response, not_found_profile, parse_tag_uuids, sync_profile_tags, validation_error,
};
use sea_orm::DatabaseConnection;

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
    db: &DatabaseConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> std::result::Result<(), Box<Response>> {
    let existing = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user_id))
        .one(db)
        .await
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
    ctx: &AppContext,
    headers: &HeaderMap,
    payload: &CreateProfileBody,
) -> std::result::Result<crate::models::_entities::users::Model, Box<Response>> {
    let (_session, user) = require_auth_db(&ctx.db, headers).await?;
    validate_profile_fields(headers, payload)?;
    check_no_existing_profile(&ctx.db, headers, user.id).await?;
    Ok(user)
}

fn uploads_unavailable(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        headers,
        ErrorSpec {
            error: "Upload storage is temporarily unavailable".to_string(),
            code: "UPLOADS_UNAVAILABLE",
            details: None,
        },
    )
}

async fn validate_profile_picture_reference(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    owner_profile_id: Option<Uuid>,
    raw_picture: &str,
) -> std::result::Result<String, Box<Response>> {
    let filename = extract_filename(raw_picture);
    if let Err(message) = validate_filename(&filename) {
        return Err(Box::new(validation_error(headers, message)));
    }

    if let Some(profile_id) = owner_profile_id {
        let owned_upload = uploads::Entity::find()
            .filter(uploads::Column::OwnerId.eq(Some(profile_id)))
            .filter(uploads::Column::Filename.eq(filename.clone()))
            .filter(uploads::Column::Deleted.eq(false))
            .one(db)
            .await
            .map_err(|_| Box::new(uploads_unavailable(headers)))?;

        if owned_upload.is_none() {
            return Err(Box::new(validation_error(
                headers,
                "Profile picture must reference your uploaded image",
            )));
        }
    }

    let exists = super::super::uploads::uploads_storage::exists(&filename)
        .await
        .map_err(|_| Box::new(uploads_unavailable(headers)))?;
    if !exists {
        return Err(Box::new(validation_error(
            headers,
            "Profile picture file was not found in upload storage",
        )));
    }

    Ok(filename)
}

fn build_create_model(
    user: &crate::models::_entities::users::Model,
    payload: &CreateProfileBody,
    profile_picture: Option<String>,
) -> (profiles::ActiveModel, Uuid) {
    let now = Utc::now();
    let profile_id = Uuid::new_v4();
    let images_json = payload.images.as_ref().and_then(|imgs| {
        serde_json::to_value(imgs.iter().map(|s| extract_filename(s)).collect::<Vec<_>>()).ok()
    });

    let model = profiles::ActiveModel {
        id: ActiveValue::Set(profile_id),
        user_id: ActiveValue::Set(user.id),
        name: ActiveValue::Set(payload.name.trim().to_string()),
        bio: ActiveValue::Set(payload.bio.clone()),
        age: ActiveValue::Set(i16::from(payload.age)),
        profile_picture: ActiveValue::Set(profile_picture),
        images: ActiveValue::Set(images_json),
        program: ActiveValue::Set(payload.program.clone()),
        gradient_start: ActiveValue::Set(payload.gradient_start.clone()),
        gradient_end: ActiveValue::Set(payload.gradient_end.clone()),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };
    (model, profile_id)
}

pub(in crate::controllers::migration_api) async fn profile_create(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateProfileBody>,
) -> Result<Response> {
    let user = match validate_create(&ctx, &headers, &payload).await {
        Ok(u) => u,
        Err(response) => return Ok(*response),
    };
    let requested_profile_picture = match payload.profile_picture.as_deref() {
        Some(raw_picture) => {
            match validate_profile_picture_reference(&ctx.db, &headers, None, raw_picture).await {
                Ok(filename) => Some(filename),
                Err(response) => return Ok(*response),
            }
        }
        None => None,
    };
    let should_sync_matrix_avatar = requested_profile_picture.is_some();

    let (model, profile_id) = build_create_model(&user, &payload, requested_profile_picture);
    let inserted = model
        .insert(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_ids = parse_tag_uuids(payload.tags.or(payload.tag_ids));
    if !tag_ids.is_empty() {
        sync_profile_tags(&ctx.db, profile_id, &tag_ids).await?;
    }

    crate::search::invalidate_search_cache();

    if should_sync_matrix_avatar {
        let user_pid = user.pid;
        if let Err(error) = enqueue_matrix_profile_avatar_sync(
            &ctx.db,
            &user_pid,
            inserted.profile_picture.as_deref(),
        )
        .await
        {
            tracing::warn!(%error, user_pid = %user_pid, "failed to enqueue matrix avatar sync after profile create");
        }
    }

    let data = full_profile_response(&ctx.db, &inserted, &user.pid).await?;
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

fn apply_profile_updates(
    profile: profiles::Model,
    payload: &UpdateProfileBody,
    profile_picture: Option<String>,
) -> profiles::ActiveModel {
    let mut active: profiles::ActiveModel = profile.into();

    if let Some(name) = &payload.name {
        active.name = ActiveValue::Set(name.trim().to_string());
    }
    if let Some(age) = payload.age {
        active.age = ActiveValue::Set(i16::from(age));
    }
    if let Some(bio) = &payload.bio {
        active.bio = ActiveValue::Set(Some(bio.clone()));
    }
    if let Some(program) = &payload.program {
        active.program = ActiveValue::Set(Some(program.clone()));
    }
    if let Some(pic) = profile_picture {
        active.profile_picture = ActiveValue::Set(Some(pic));
    }
    if let Some(images) = &payload.images {
        let filenames: Vec<String> = images.iter().map(|s| extract_filename(s)).collect();
        active.images = ActiveValue::Set(serde_json::to_value(filenames).ok());
    }
    if let Some(gs) = &payload.gradient_start {
        active.gradient_start = ActiveValue::Set(if gs.is_empty() {
            None
        } else {
            Some(gs.clone())
        });
    }
    if let Some(ge) = &payload.gradient_end {
        active.gradient_end = ActiveValue::Set(if ge.is_empty() {
            None
        } else {
            Some(ge.clone())
        });
    }
    active.updated_at = ActiveValue::Set(Utc::now().into());

    active
}

async fn load_and_verify_profile(
    ctx: &AppContext,
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(profiles::Model, crate::models::_entities::users::Model), Box<Response>> {
    let (_session, user) = require_auth_db(&ctx.db, headers).await?;
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

    let profile = profiles::Entity::find_by_id(profile_uuid)
        .one(&ctx.db)
        .await
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
    db: &DatabaseConnection,
    profile_id: Uuid,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> std::result::Result<(), crate::error::AppError> {
    if tags.is_some() || tag_ids.is_some() {
        let resolved = parse_tag_uuids(tags.or(tag_ids));
        sync_profile_tags(db, profile_id, &resolved).await?;
    }
    Ok(())
}

pub(in crate::controllers::migration_api) async fn profile_update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileBody>,
) -> Result<Response> {
    let (profile, user) = match load_and_verify_profile(&ctx, &headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };
    if let Err(response) = validate_update_payload(&headers, &payload) {
        return Ok(*response);
    }
    let requested_profile_picture = match payload.profile_picture.as_deref() {
        Some(raw_picture) => {
            match validate_profile_picture_reference(
                &ctx.db,
                &headers,
                Some(profile.id),
                raw_picture,
            )
            .await
            {
                Ok(filename) => Some(filename),
                Err(response) => return Ok(*response),
            }
        }
        None => None,
    };
    let should_sync_matrix_avatar = requested_profile_picture.is_some();

    let profile_uuid = profile.id;
    let active = apply_profile_updates(profile, &payload, requested_profile_picture);

    let updated = active
        .update(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    maybe_sync_tags(&ctx.db, profile_uuid, payload.tags, payload.tag_ids).await?;

    crate::search::invalidate_search_cache();

    if should_sync_matrix_avatar {
        let user_pid = user.pid;
        if let Err(error) = enqueue_matrix_profile_avatar_sync(
            &ctx.db,
            &user_pid,
            updated.profile_picture.as_deref(),
        )
        .await
        {
            tracing::warn!(%error, user_pid = %user_pid, "failed to enqueue matrix avatar sync after profile update");
        }
    }

    let data = full_profile_response(&ctx.db, &updated, &user.pid).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::controllers::migration_api) async fn profile_delete(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (profile, _user) = match load_and_verify_profile(&ctx, &headers, &id).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
    };

    profiles::Entity::delete_by_id(profile.id)
        .exec(&ctx.db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    crate::search::invalidate_search_cache();

    Ok(Json(SuccessResponse { success: true }).into_response())
}
