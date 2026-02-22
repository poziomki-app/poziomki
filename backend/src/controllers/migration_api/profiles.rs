#[path = "profiles_mutations.rs"]
mod profiles_mutations;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::{
    error_response, resolve_image_url, resolve_image_urls,
    state::{
        require_auth_db, DataResponse, FullProfileResponse, ProfileResponse, TagResponse, TagScope,
    },
    ErrorSpec,
};
use crate::db::models::profile_tags::ProfileTag;
use crate::db::models::profiles::Profile;
use crate::db::models::tags::Tag;
use crate::db::models::users::User;
use crate::db::schema::{profile_tags, profiles, tags, users};

pub(super) use profiles_mutations::{profile_create, profile_delete, profile_update};

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

fn not_found_profile(headers: &HeaderMap, id: &str) -> Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: format!("Profile '{id}' not found"),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

fn validation_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

async fn profile_to_response(profile: &Profile, user_pid: &Uuid) -> ProfileResponse {
    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let raw_images: Vec<String> = profile
        .images
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default();
    let images = resolve_image_urls(&raw_images).await;

    ProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        images,
        program: profile.program.clone(),
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

async fn load_profile_tags(
    profile_id: Uuid,
) -> std::result::Result<Vec<TagResponse>, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_links = profile_tags::table
        .filter(profile_tags::profile_id.eq(profile_id))
        .load::<ProfileTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = tag_links.iter().map(|link| link.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_models = tags::table
        .filter(tags::id.eq_any(&tag_ids))
        .load::<Tag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    Ok(tag_models
        .iter()
        .map(|t| TagResponse {
            id: t.id.to_string(),
            name: t.name.clone(),
            scope: scope_from_str(&t.scope),
            category: t.category.clone(),
            emoji: t.emoji.clone(),
            onboarding_order: t.onboarding_order.clone(),
        })
        .collect())
}

async fn full_profile_response(
    profile: &Profile,
    user_pid: &Uuid,
) -> std::result::Result<FullProfileResponse, crate::error::AppError> {
    let profile_tags = load_profile_tags(profile.id).await?;

    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let raw_images: Vec<String> = profile
        .images
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default();
    let images = resolve_image_urls(&raw_images).await;

    Ok(FullProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        images,
        program: profile.program.clone(),
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        tags: profile_tags,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}

async fn sync_profile_tags(
    profile_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    diesel::delete(profile_tags::table.filter(profile_tags::profile_id.eq(profile_id)))
        .execute(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let new_tags: Vec<ProfileTag> = tag_ids
        .iter()
        .map(|tag_id| ProfileTag {
            profile_id,
            tag_id: *tag_id,
        })
        .collect();

    if !new_tags.is_empty() {
        diesel::insert_into(profile_tags::table)
            .values(&new_tags)
            .execute(&mut conn)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
    }

    Ok(())
}

fn parse_tag_uuids(raw: Option<Vec<String>>) -> Vec<Uuid> {
    raw.unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect()
}

pub(super) async fn profile_me(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let data = match profile {
        Some(ref p) => Some(full_profile_response(p, &user.pid).await?),
        None => None,
    };

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let profile = profiles::table
        .find(profile_uuid)
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let Some(profile) = profile else {
        return Ok(not_found_profile(&headers, &id));
    };

    let owner = users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let user_pid = owner.map_or(Uuid::nil(), |u| u.pid);

    let data = profile_to_response(&profile, &user_pid).await;
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get_full(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let profile = profiles::table
        .find(profile_uuid)
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let Some(profile) = profile else {
        return Ok(not_found_profile(&headers, &id));
    };

    let owner = users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let user_pid = owner.map_or(Uuid::nil(), |u| u.pid);

    let data = full_profile_response(&profile, &user_pid).await?;
    Ok(Json(DataResponse { data }).into_response())
}
