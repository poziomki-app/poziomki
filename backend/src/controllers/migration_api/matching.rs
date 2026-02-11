use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::{
    resolve_image_url,
    state::{
        require_auth_db, DataResponse, MatchingQuery, MatchingTagResponse, ProfileRecommendation,
        TagScope,
    },
};
use crate::models::_entities::{profile_tags, profiles, tags, users};

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

async fn load_profile_tags(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<MatchingTagResponse>, loco_rs::Error> {
    let tag_links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = tag_links.iter().map(|l| l.tag_id).collect();
    let profile_tag_models = if tag_ids.is_empty() {
        vec![]
    } else {
        tags::Entity::find()
            .filter(tags::Column::Id.is_in(tag_ids))
            .all(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    };

    let matched_tags = profile_tag_models
        .iter()
        .map(|t| MatchingTagResponse {
            id: t.id.to_string(),
            name: t.name.clone(),
            scope: scope_from_str(&t.scope),
        })
        .collect::<Vec<_>>();

    Ok(matched_tags)
}

async fn load_users_by_ids(
    db: &DatabaseConnection,
    user_ids: &[i32],
) -> std::result::Result<Vec<users::Model>, loco_rs::Error> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.to_vec()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))
}

async fn build_recommendation(
    db: &DatabaseConnection,
    profile: &profiles::Model,
    user_pid: Uuid,
) -> std::result::Result<ProfileRecommendation, loco_rs::Error> {
    let matched_tags = load_profile_tags(db, profile.id).await?;
    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };
    Ok(ProfileRecommendation {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        program: profile.program.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
        tags: matched_tags,
    })
}

pub(super) async fn profiles_recommendations(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = u64::from(query.limit.unwrap_or(10).clamp(1, 50));

    let other_profiles = profiles::Entity::find()
        .filter(profiles::Column::UserId.ne(user.id))
        .order_by_desc(profiles::Column::CreatedAt)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let user_ids: Vec<i32> = other_profiles.iter().map(|p| p.user_id).collect();
    let user_models = load_users_by_ids(&ctx.db, &user_ids).await?;

    let mut data = Vec::new();
    for profile in &other_profiles {
        let user_pid = user_models
            .iter()
            .find(|u| u.id == profile.user_id)
            .map_or(Uuid::nil(), |u| u.pid);
        data.push(build_recommendation(&ctx.db, profile, user_pid).await?);
    }

    Ok(Json(DataResponse { data }).into_response())
}
