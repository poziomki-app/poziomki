use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use crate::controllers::migration_api::{
    error_response, extract_filename,
    state::{
        require_auth_db, validate_profile_age, validate_profile_name, CreateProfileBody,
        DataResponse, SuccessResponse, UpdateProfileBody,
    },
    ErrorSpec,
};
use crate::models::_entities::profiles;

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
            Box::new(error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: format!("Database error: {e}"),
                    code: "DATABASE_ERROR",
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

fn build_create_model(
    user: &crate::models::_entities::users::Model,
    payload: &CreateProfileBody,
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
        profile_picture: ActiveValue::Set(payload.profile_picture.as_deref().map(extract_filename)),
        images: ActiveValue::Set(images_json),
        program: ActiveValue::Set(payload.program.clone()),
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

    let (model, profile_id) = build_create_model(&user, &payload);
    let inserted = model
        .insert(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let tag_ids = parse_tag_uuids(payload.tags.or(payload.tag_ids));
    if !tag_ids.is_empty() {
        sync_profile_tags(&ctx.db, profile_id, &tag_ids).await?;
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
    Ok(())
}

fn apply_profile_updates(
    profile: profiles::Model,
    payload: &UpdateProfileBody,
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
    if let Some(pic) = &payload.profile_picture {
        active.profile_picture = ActiveValue::Set(Some(extract_filename(pic)));
    }
    if let Some(images) = &payload.images {
        let filenames: Vec<String> = images.iter().map(|s| extract_filename(s)).collect();
        active.images = ActiveValue::Set(serde_json::to_value(filenames).ok());
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
) -> std::result::Result<(), loco_rs::Error> {
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
    validate_update_payload(&headers, &payload)
        .map_err(|r| loco_rs::Error::Message(format!("{r:?}")))?;

    let profile_uuid = profile.id;
    let active = apply_profile_updates(profile, &payload);

    let updated = active
        .update(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    maybe_sync_tags(&ctx.db, profile_uuid, payload.tags, payload.tag_ids).await?;

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
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    Ok(Json(SuccessResponse { success: true }).into_response())
}
