#[path = "profiles_mutations.rs"]
mod profiles_mutations;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use super::{
    error_response,
    state::{
        require_auth_db, DataResponse, FullProfileResponse, ProfileResponse, TagResponse, TagScope,
    },
    ErrorSpec,
};
use crate::models::_entities::{profile_tags, profiles, tags};

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

fn profile_to_response(profile: &profiles::Model, user_pid: &Uuid) -> ProfileResponse {
    ProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture: profile.profile_picture.clone(),
        images: profile
            .images
            .as_ref()
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .unwrap_or_default(),
        program: profile.program.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

async fn load_profile_tags(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<TagResponse>, loco_rs::Error> {
    let tag_links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = tag_links.iter().map(|link| link.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_models = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

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
    db: &DatabaseConnection,
    profile: &profiles::Model,
    user_pid: &Uuid,
) -> std::result::Result<FullProfileResponse, loco_rs::Error> {
    let profile_tags = load_profile_tags(db, profile.id).await?;
    Ok(FullProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture: profile.profile_picture.clone(),
        images: profile
            .images
            .as_ref()
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .unwrap_or_default(),
        program: profile.program.clone(),
        tags: profile_tags,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}

async fn sync_profile_tags(
    db: &DatabaseConnection,
    profile_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), loco_rs::Error> {
    profile_tags::Entity::delete_many()
        .filter(profile_tags::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    for tag_id in tag_ids {
        let link = profile_tags::ActiveModel {
            profile_id: ActiveValue::Set(profile_id),
            tag_id: ActiveValue::Set(*tag_id),
        };
        link.insert(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;
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
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let data = match profile {
        Some(ref p) => Some(full_profile_response(&ctx.db, p, &user.pid).await?),
        None => None,
    };

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| loco_rs::Error::Message("Invalid profile ID".to_string()))?;

    let profile = profiles::Entity::find_by_id(profile_uuid)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let Some(profile) = profile else {
        return Ok(not_found_profile(&headers, &id));
    };

    let owner = crate::models::_entities::users::Entity::find_by_id(profile.user_id)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    let user_pid = owner.map_or(Uuid::nil(), |u| u.pid);

    let data = profile_to_response(&profile, &user_pid);
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get_full(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| loco_rs::Error::Message("Invalid profile ID".to_string()))?;

    let profile = profiles::Entity::find_by_id(profile_uuid)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let Some(profile) = profile else {
        return Ok(not_found_profile(&headers, &id));
    };

    let owner = crate::models::_entities::users::Entity::find_by_id(profile.user_id)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    let user_pid = owner.map_or(Uuid::nil(), |u| u.pid);

    let data = full_profile_response(&ctx.db, &profile, &user_pid).await?;
    Ok(Json(DataResponse { data }).into_response())
}
