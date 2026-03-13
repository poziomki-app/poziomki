use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::matching_repo::MatchingRepository;
use crate::api::state::{MatchingTagResponse, ProfileRecommendation};
use crate::api::{resolve_image_urls, resolve_thumbhashes};
use crate::db::models::profiles::Profile;
use crate::db::models::user_settings::UserSetting;
use crate::db::models::users::User;
use crate::db::schema::user_settings;

struct RecommendationContext<'a> {
    user_models: &'a [User],
    pic_map: &'a HashMap<String, String>,
    thumbhash_map: &'a HashMap<String, String>,
    privacy_map: &'a HashMap<i32, bool>,
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
    let thumbhash = profile
        .profile_picture
        .as_ref()
        .and_then(|pic| ctx.thumbhash_map.get(pic))
        .cloned();
    let show_program = ctx
        .privacy_map
        .get(&profile.user_id)
        .copied()
        .unwrap_or(false);
    let program = if show_program {
        profile.program.clone()
    } else {
        None
    };
    ProfileRecommendation {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        profile_picture,
        thumbhash,
        program,
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
        tags: profile_tags,
        score,
    }
}

async fn batch_load_show_program(
    user_ids: &[i32],
    conn: &mut crate::db::DbConn,
) -> crate::error::AppResult<HashMap<i32, bool>> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let settings: Vec<UserSetting> = user_settings::table
        .filter(user_settings::user_id.eq_any(user_ids))
        .load(conn)
        .await?;
    Ok(settings
        .into_iter()
        .map(|s| (s.user_id, s.privacy_show_program))
        .collect())
}

pub(super) async fn build_recommendations_response(
    top: &[(f64, &Profile)],
    repo: &MatchingRepository,
    conn: &mut crate::db::DbConn,
) -> std::result::Result<Vec<ProfileRecommendation>, crate::error::AppError> {
    let user_ids: Vec<i32> = top.iter().map(|(_, p)| p.user_id).collect();
    let user_models = repo.load_users_by_ids(&user_ids, conn).await?;

    let top_ids: Vec<Uuid> = top.iter().map(|(_, p)| p.id).collect();
    let top_tags = repo.batch_load_profile_tags(&top_ids, conn).await?;

    let privacy_map = batch_load_show_program(&user_ids, conn).await?;

    let pic_filenames: Vec<String> = top
        .iter()
        .filter_map(|(_, p)| p.profile_picture.clone())
        .collect();

    let (resolved_pics, thumbhash_map) = tokio::join!(
        resolve_image_urls(&pic_filenames),
        resolve_thumbhashes(&pic_filenames),
    );
    let pic_map: HashMap<String, String> = pic_filenames.into_iter().zip(resolved_pics).collect();

    let ctx = RecommendationContext {
        user_models: &user_models,
        pic_map: &pic_map,
        thumbhash_map: &thumbhash_map,
        privacy_map: &privacy_map,
    };
    Ok(top
        .iter()
        .map(|(score, profile)| {
            let profile_tags = top_tags.get(&profile.id).cloned().unwrap_or_default();
            build_profile_recommendation(*score, profile, &ctx, profile_tags)
        })
        .collect())
}
