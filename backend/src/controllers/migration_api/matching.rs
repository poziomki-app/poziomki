use axum::{extract::Query, http::HeaderMap, response::IntoResponse, Json};
use loco_rs::prelude::*;

use super::state::{
    bounded_matching_limit, lock_state, require_auth, DataResponse, MatchingQuery,
    MatchingTagResponse, ProfileRecommendation,
};

pub(super) async fn profiles_recommendations(
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let mut state = lock_state();
    let (_session, user) = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = bounded_matching_limit(query.limit);
    let mut profile_ids = state
        .profiles
        .values()
        .filter(|profile| profile.user_id != user.id)
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();

    profile_ids.sort_by(|left, right| {
        state
            .profiles
            .get(left)
            .zip(state.profiles.get(right))
            .map_or(std::cmp::Ordering::Equal, |(l, r)| {
                r.created_at.cmp(&l.created_at)
            })
    });

    let data = profile_ids
        .into_iter()
        .take(limit)
        .filter_map(|profile_id| state.profiles.get(&profile_id))
        .map(|profile| {
            let tags = profile
                .tag_ids
                .iter()
                .filter_map(|tag_id| state.tags.get(tag_id))
                .map(|tag| MatchingTagResponse {
                    id: tag.id.clone(),
                    name: tag.name.clone(),
                    scope: tag.scope,
                })
                .collect::<Vec<_>>();

            ProfileRecommendation {
                id: profile.id.clone(),
                user_id: profile.user_id.clone(),
                name: profile.name.clone(),
                bio: profile.bio.clone(),
                age: profile.age,
                profile_picture: profile.profile_picture.clone(),
                program: profile.program.clone(),
                created_at: profile.created_at.to_rfc3339(),
                updated_at: profile.updated_at.to_rfc3339(),
                tags,
            }
        })
        .collect::<Vec<_>>();
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}
