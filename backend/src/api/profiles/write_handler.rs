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

use super::{full_profile_response, parse_tag_uuids, sync_profile_tags};
use crate::api::{
    extract_filename,
    state::{CreateProfileBody, DataResponse, SuccessResponse, UpdateProfileBody},
};
use crate::db::models::profiles::{NewProfile, Profile};
use crate::db::schema::profiles;

#[path = "write_service.rs"]
mod write_service;
use write_service::{
    build_update_changeset, load_and_verify_profile, resolve_picture_filename,
    validate_and_prepare_update, validate_create,
};

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

async fn insert_profile(
    new_profile: &NewProfile,
    profile_id: Uuid,
    payload: &CreateProfileBody,
) -> Result<Profile> {
    let mut conn = crate::db::conn().await?;
    let inserted = diesel::insert_into(profiles::table)
        .values(new_profile)
        .get_result::<Profile>(&mut conn)
        .await?;

    let tag_ids = parse_tag_uuids(payload.tags.clone().or_else(|| payload.tag_ids.clone()));
    if !tag_ids.is_empty() {
        sync_profile_tags(profile_id, &tag_ids).await?;
    }

    Ok(inserted)
}

pub(in crate::api) async fn profile_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateProfileBody>,
) -> Result<Response> {
    let user = match validate_create(&headers, &payload).await {
        Ok(u) => u,
        Err(response) => return Ok(*response),
    };
    let picture =
        match resolve_picture_filename(&headers, None, payload.profile_picture.as_deref()).await {
            Ok(p) => p,
            Err(response) => return Ok(*response),
        };

    let (new_profile, profile_id) = build_create_model(&user, &payload, picture);
    let inserted = insert_profile(&new_profile, profile_id, &payload).await?;

    let format = crate::api::image_format_from_headers(&headers);
    let data = full_profile_response(&inserted, &user.pid, Some(user.id), format).await?;
    Ok((axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response())
}

async fn apply_update(
    profile: &Profile,
    payload: &UpdateProfileBody,
    changeset: crate::db::models::profiles::ProfileChangeset,
) -> Result<Profile> {
    let mut conn = crate::db::conn().await?;
    let updated = diesel::update(profiles::table.find(profile.id))
        .set(&changeset)
        .get_result::<Profile>(&mut conn)
        .await?;

    if payload.tags.is_some() || payload.tag_ids.is_some() {
        let resolved = parse_tag_uuids(payload.tags.clone().or_else(|| payload.tag_ids.clone()));
        sync_profile_tags(profile.id, &resolved).await?;
    }

    Ok(updated)
}

pub(in crate::api) async fn profile_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileBody>,
) -> Result<Response> {
    let (profile, user, picture) = match validate_and_prepare_update(&headers, &id, &payload).await
    {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };
    let changeset = build_update_changeset(&payload, picture);
    let updated = apply_update(&profile, &payload, changeset).await?;

    let format = crate::api::image_format_from_headers(&headers);
    let data = full_profile_response(&updated, &user.pid, Some(user.id), format).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::api) async fn profile_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (profile, _user) = match load_and_verify_profile(&headers, &id).await {
        Ok(p) => p,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn().await?;
    diesel::delete(profiles::table.find(profile.id))
        .execute(&mut conn)
        .await?;

    Ok(Json(SuccessResponse { success: true }).into_response())
}
