use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::state::{
    require_auth_db, DataResponse, MatchingQuery, MatchingTagResponse, ProfileRecommendation,
    TagScope,
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

    // Get profiles that don't belong to this user, ordered by newest first
    let other_profiles = profiles::Entity::find()
        .filter(profiles::Column::UserId.ne(user.id))
        .order_by_desc(profiles::Column::CreatedAt)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    // Collect user IDs to fetch pids
    let user_ids: Vec<i32> = other_profiles.iter().map(|p| p.user_id).collect();
    let user_models = if user_ids.is_empty() {
        vec![]
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids))
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    };

    let mut data = Vec::new();
    for profile in &other_profiles {
        let user_pid = user_models
            .iter()
            .find(|u| u.id == profile.user_id)
            .map_or(Uuid::nil(), |u| u.pid);

        let matched_tags = load_profile_tags(&ctx.db, profile.id).await?;

        data.push(ProfileRecommendation {
            id: profile.id.to_string(),
            user_id: user_pid.to_string(),
            name: profile.name.clone(),
            bio: profile.bio.clone(),
            age: u8::try_from(profile.age).unwrap_or(0),
            profile_picture: profile.profile_picture.clone(),
            program: profile.program.clone(),
            created_at: profile.created_at.to_rfc3339(),
            updated_at: profile.updated_at.to_rfc3339(),
            tags: matched_tags,
        });
    }

    Ok(Json(DataResponse { data }).into_response())
}
