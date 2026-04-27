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

/// DB-only context for the top-N scored profiles. Collected inside the viewer
/// transaction so RLS applies; external image / thumbhash resolution happens
/// in `finalize_recommendations` after the tx closes.
pub(super) struct RecommendationData {
    pub(super) scored: Vec<(f64, Profile)>,
    pub(super) user_models: Vec<User>,
    pub(super) profile_tags: HashMap<Uuid, Vec<MatchingTagResponse>>,
    pub(super) privacy_map: HashMap<i32, bool>,
}

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
        .unwrap_or(true);
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
        status: profile.status_text.clone(),
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

pub(super) async fn batch_load_show_program(
    user_ids: &[i32],
    conn: &mut diesel_async::AsyncPgConnection,
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

/// Load the DB-backed fields needed to render a recommendations payload.
/// Must run inside a viewer-scoped transaction. `finalize_recommendations`
/// converts the result into the HTTP response shape.
pub(super) async fn load_recommendation_data(
    top: &[(f64, &Profile)],
    repo: &MatchingRepository,
    conn: &mut diesel_async::AsyncPgConnection,
    privacy_map: &HashMap<i32, bool>,
) -> std::result::Result<RecommendationData, crate::error::AppError> {
    let user_ids: Vec<i32> = top.iter().map(|(_, p)| p.user_id).collect();
    let user_models = repo.load_users_by_ids(&user_ids, conn).await?;

    let top_ids: Vec<Uuid> = top.iter().map(|(_, p)| p.id).collect();
    let profile_tags = repo.batch_load_profile_tags(&top_ids, conn).await?;

    let scored: Vec<(f64, Profile)> = top
        .iter()
        .map(|(score, profile)| (*score, (*profile).clone()))
        .collect();

    Ok(RecommendationData {
        scored,
        user_models,
        profile_tags,
        privacy_map: privacy_map.clone(),
    })
}

/// Resolve signed image URLs / thumbhashes and build the final response. Runs
/// outside the DB transaction so imgproxy latency doesn't hold a connection.
pub(super) async fn finalize_recommendations(
    data: RecommendationData,
) -> Vec<ProfileRecommendation> {
    let pic_filenames: Vec<String> = data
        .scored
        .iter()
        .filter_map(|(_, p)| p.profile_picture.clone())
        .collect();

    let (resolved_pics, thumbhash_map) = tokio::join!(
        resolve_image_urls(&pic_filenames),
        resolve_thumbhashes(&pic_filenames),
    );
    let pic_map: HashMap<String, String> = pic_filenames.into_iter().zip(resolved_pics).collect();

    let ctx = RecommendationContext {
        user_models: &data.user_models,
        pic_map: &pic_map,
        thumbhash_map: &thumbhash_map,
        privacy_map: &data.privacy_map,
    };
    data.scored
        .iter()
        .map(|(score, profile)| {
            let profile_tags = data
                .profile_tags
                .get(&profile.id)
                .cloned()
                .unwrap_or_default();
            build_profile_recommendation(*score, profile, &ctx, profile_tags)
        })
        .collect()
}
