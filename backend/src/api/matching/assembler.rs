use std::collections::HashMap;

use uuid::Uuid;

use super::matching_repo::MatchingRepository;
use crate::api::resolve_image_urls;
use crate::api::state::{MatchingTagResponse, ProfileRecommendation};
use crate::db::models::profiles::Profile;
use crate::db::models::users::User;

struct RecommendationContext<'a> {
    user_models: &'a [User],
    pic_map: &'a HashMap<String, String>,
}

fn build_profile_recommendation(
    score: f64,
    profile: &Profile,
    ctx: &RecommendationContext<'_>,
    profile_tags: Vec<MatchingTagResponse>,
) -> ProfileRecommendation {
    let user_pid = ctx
        .user_models
        .iter()
        .find(|u| u.id == profile.user_id)
        .map_or(Uuid::nil(), |u| u.pid);
    let profile_picture = profile
        .profile_picture
        .as_ref()
        .and_then(|pic| ctx.pic_map.get(pic))
        .cloned();
    ProfileRecommendation {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        program: profile.program.clone(),
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
        tags: profile_tags,
        score,
    }
}

pub(super) async fn build_recommendations_response(
    top: &[(f64, &Profile)],
    repo: &MatchingRepository,
) -> std::result::Result<Vec<ProfileRecommendation>, crate::error::AppError> {
    let user_ids: Vec<i32> = top.iter().map(|(_, p)| p.user_id).collect();
    let user_models = repo.load_users_by_ids(&user_ids).await?;

    let top_ids: Vec<Uuid> = top.iter().map(|(_, p)| p.id).collect();
    let top_tags = repo.batch_load_profile_tags(&top_ids).await?;

    let pic_urls: Vec<String> = top
        .iter()
        .filter_map(|(_, p)| p.profile_picture.clone())
        .collect();
    let resolved_pics = resolve_image_urls(&pic_urls).await;
    let pic_map: HashMap<String, String> = pic_urls.into_iter().zip(resolved_pics).collect();

    let ctx = RecommendationContext {
        user_models: &user_models,
        pic_map: &pic_map,
    };
    Ok(top
        .iter()
        .map(|(score, profile)| {
            let profile_tags = top_tags.get(&profile.id).cloned().unwrap_or_default();
            build_profile_recommendation(*score, profile, &ctx, profile_tags)
        })
        .collect())
}
