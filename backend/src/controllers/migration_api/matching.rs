use std::collections::{HashMap, HashSet};

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::{
    require_auth_db, DataResponse, MatchingQuery, MatchingTagResponse, ProfileRecommendation,
    TagScope,
};
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profile_tags::ProfileTag;
use crate::db::models::profiles::Profile;
use crate::db::models::tags::Tag;
use crate::db::models::users::User;
use crate::db::schema::{event_tags, events, profile_tags, profiles, tags, users};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

async fn load_profile_tag_ids(
    profile_id: Uuid,
) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    let tag_links = profile_tags::table
        .filter(profile_tags::profile_id.eq(profile_id))
        .load::<ProfileTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    Ok(tag_links.iter().map(|l| l.tag_id).collect())
}

/// Batch-load all tag links and tag models for a set of profile IDs in 2 queries.
/// Returns a map from `profile_id` -> `Vec<MatchingTagResponse>`.
async fn batch_load_profile_tags(
    profile_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Vec<MatchingTagResponse>>, crate::error::AppError> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let all_links = profile_tags::table
        .filter(profile_tags::profile_id.eq_any(profile_ids))
        .load::<ProfileTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let all_tag_ids: HashSet<Uuid> = all_links.iter().map(|l| l.tag_id).collect();
    let tag_models = if all_tag_ids.is_empty() {
        vec![]
    } else {
        tags::table
            .filter(tags::id.eq_any(&all_tag_ids.into_iter().collect::<Vec<_>>()))
            .load::<Tag>(&mut conn)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?
    };

    let tag_by_id: HashMap<Uuid, &Tag> = tag_models.iter().map(|t| (t.id, t)).collect();

    let mut result: HashMap<Uuid, Vec<MatchingTagResponse>> = HashMap::new();
    for link in &all_links {
        if let Some(tag) = tag_by_id.get(&link.tag_id) {
            result
                .entry(link.profile_id)
                .or_default()
                .push(MatchingTagResponse {
                    id: tag.id.to_string(),
                    name: tag.name.clone(),
                    scope: scope_from_str(&tag.scope),
                });
        }
    }
    Ok(result)
}

/// Batch-load tag ID sets for a set of profile IDs in 1 query.
async fn batch_load_profile_tag_ids(
    profile_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, crate::error::AppError> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let all_links = profile_tags::table
        .filter(profile_tags::profile_id.eq_any(profile_ids))
        .load::<ProfileTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let mut result: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for link in &all_links {
        result
            .entry(link.profile_id)
            .or_default()
            .insert(link.tag_id);
    }
    Ok(result)
}

async fn load_users_by_ids(
    user_ids: &[i32],
) -> std::result::Result<Vec<User>, crate::error::AppError> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    users::table
        .filter(users::id.eq_any(user_ids))
        .load::<User>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))
}

/// Haversine distance between two (lat, lng) points in kilometres.
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

/// Returns a 0.0-1.0 score: 1.0 when distance is 0, 0.0 at or beyond `max_km`.
fn proximity_score(distance_km: f64, max_km: f64) -> f64 {
    if max_km <= 0.0 {
        return 0.0;
    }
    (1.0 - distance_km / max_km).clamp(0.0, 1.0)
}

#[allow(clippy::cast_precision_loss)]
fn jaccard(a: &HashSet<Uuid>, b: &HashSet<Uuid>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    intersection as f64 / union as f64
}

pub(super) async fn profiles_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(10).clamp(1, 50) as usize;

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    // Load current user's profile and tags
    let my_profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let my_tag_ids = match &my_profile {
        Some(p) => load_profile_tag_ids(p.id).await?,
        None => HashSet::new(),
    };

    // Fetch candidate profiles (more than limit so we can score and rank)
    let candidates = profiles::table
        .filter(profiles::user_id.ne(user.id))
        .order(profiles::created_at.desc())
        .limit(200)
        .load::<Profile>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    // Batch-load all candidate tag IDs in one query
    let candidate_ids: Vec<Uuid> = candidates.iter().map(|c| c.id).collect();
    let all_candidate_tags = batch_load_profile_tag_ids(&candidate_ids).await?;

    // Score each candidate
    let mut scored: Vec<(f64, &Profile)> = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let candidate_tags = all_candidate_tags
            .get(&candidate.id)
            .cloned()
            .unwrap_or_default();
        let mut score = jaccard(&my_tag_ids, &candidate_tags) * 100.0;

        // Bonus for same program
        if let Some(my_prog) = my_profile.as_ref().and_then(|p| p.program.as_ref()) {
            if candidate.program.as_deref() == Some(my_prog.as_str()) {
                score += 10.0;
            }
        }

        scored.push((score, candidate));
    }

    // Sort by score DESC, break ties by created_at DESC
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.created_at.cmp(&a.1.created_at))
    });

    // Take top `limit` results
    let top = scored.into_iter().take(limit).collect::<Vec<_>>();

    let user_ids: Vec<i32> = top.iter().map(|(_, p)| p.user_id).collect();
    let user_models = load_users_by_ids(&user_ids).await?;

    // Batch-load tags and resolve images for the top profiles
    let top_ids: Vec<Uuid> = top.iter().map(|(_, p)| p.id).collect();
    let top_tags = batch_load_profile_tags(&top_ids).await?;

    let pic_urls: Vec<String> = top
        .iter()
        .filter_map(|(_, p)| p.profile_picture.clone())
        .collect();
    let resolved_pics = super::resolve_image_urls(&pic_urls).await;
    let pic_map: HashMap<String, String> = pic_urls.into_iter().zip(resolved_pics).collect();

    let data: Vec<ProfileRecommendation> = top
        .iter()
        .map(|(score, profile)| {
            let user_pid = user_models
                .iter()
                .find(|u| u.id == profile.user_id)
                .map_or(Uuid::nil(), |u| u.pid);
            let profile_picture = profile
                .profile_picture
                .as_ref()
                .and_then(|pic| pic_map.get(pic))
                .cloned();
            let profile_tags = top_tags.get(&profile.id).cloned().unwrap_or_default();
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
                score: *score,
            }
        })
        .collect();

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

/// Batch-load tag ID sets for a set of event IDs in 1 query.
async fn batch_load_event_tag_ids(
    event_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, crate::error::AppError> {
    if event_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let all_links = event_tags::table
        .filter(event_tags::event_id.eq_any(event_ids))
        .load::<EventTag>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let mut result: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for link in &all_links {
        result.entry(link.event_id).or_default().insert(link.tag_id);
    }
    Ok(result)
}

pub(super) async fn events_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100) as usize;

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    // Load current user's profile and tags
    let my_profile = profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let my_profile_id = my_profile.as_ref().map_or(Uuid::nil(), |p| p.id);

    let my_tag_ids = match &my_profile {
        Some(p) => load_profile_tag_ids(p.id).await?,
        None => HashSet::new(),
    };

    let now = Utc::now();

    // Fetch future events
    let future_events = events::table
        .filter(events::starts_at.ge(now))
        .order(events::starts_at.asc())
        .limit(100)
        .load::<Event>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    // Batch-load all event tag IDs in one query
    let event_ids: Vec<Uuid> = future_events.iter().map(|e| e.id).collect();
    let all_event_tags = batch_load_event_tag_ids(&event_ids).await?;

    // Resolve user geo query: lat, lng, max radius in km (default 20 km)
    let user_geo = query.lat.zip(query.lng).map(|(lat, lng)| {
        (
            lat,
            lng,
            f64::from(query.radius_m.unwrap_or(20_000)) / 1000.0,
        )
    });

    // Score each event by user tag overlap + optional proximity.
    // Without geo: pure tag relevance (0-100).
    // With geo: tags still dominate (85 %) with proximity as mild tiebreaker (15 %).
    let mut scored: Vec<(f64, &Event)> = Vec::with_capacity(future_events.len());
    for event in &future_events {
        let event_tag_ids = all_event_tags.get(&event.id).cloned().unwrap_or_default();
        #[allow(clippy::cast_precision_loss)]
        let tag_score = if my_tag_ids.is_empty() {
            0.0
        } else {
            let shared = my_tag_ids.intersection(&event_tag_ids).count();
            (shared as f64 / my_tag_ids.len() as f64) * 100.0
        };

        let score = if let Some((ulat, ulng, max_km)) = user_geo {
            let geo_bonus = match (event.latitude, event.longitude) {
                (Some(elat), Some(elng)) => {
                    proximity_score(haversine_km(ulat, ulng, elat, elng), max_km) * 15.0
                }
                _ => 0.0,
            };
            tag_score * 0.85 + geo_bonus
        } else {
            // No location provided - pure tag relevance, no penalty
            tag_score
        };

        scored.push((score, event));
    }

    // Sort by score DESC, then starts_at ASC as tiebreaker
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.starts_at.cmp(&b.1.starts_at))
    });

    let top = scored.into_iter().take(limit).collect::<Vec<_>>();

    let top_models: Vec<Event> = top.iter().map(|(_, event)| (*event).clone()).collect();
    let base = super::events::build_event_responses(&top_models, &my_profile_id).await?;
    let score_by_event: HashMap<String, f64> = top
        .iter()
        .map(|(score, event)| (event.id.to_string(), *score))
        .collect();
    let data = base
        .into_iter()
        .map(|mut event| {
            event.score = score_by_event.get(&event.id).copied();
            event
        })
        .collect::<Vec<_>>();

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used, clippy::suboptimal_flops)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn jaccard_partial_overlap() {
        // {1,2,3} intersect {2,3,4} = {2,3}  ->  2/4 = 0.5
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(3), id(4)].into();
        assert!((jaccard(&a, &b) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn haversine_krakow_to_warsaw() {
        let dist = haversine_km(50.06, 19.94, 52.23, 21.01);
        assert!(dist > 240.0 && dist < 260.0, "got {dist}");
    }

    #[test]
    fn proximity_at_zero() {
        assert!((proximity_score(0.0, 10.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proximity_at_max() {
        assert!((proximity_score(10.0, 10.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proximity_beyond_max() {
        assert!((proximity_score(15.0, 10.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_combined_score_with_geo_max() {
        // Perfect tags + perfect proximity -> 85 + 15 = 100
        let tag_score: f64 = 100.0;
        let geo_bonus = proximity_score(0.0, 20.0) * 15.0;
        let combined = tag_score * 0.85 + geo_bonus;
        assert!((combined - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_is_asymmetric() {
        // User has 2 tags, event has 10 tags, 2 overlap -> 100%
        let user_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let event_tags: HashSet<Uuid> = (1..=10).map(id).collect();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        assert!((score - 100.0).abs() < f64::EPSILON);

        // Flip: user has 10 tags, event has 2 tags, 2 overlap -> 20%
        let user_tags2: HashSet<Uuid> = (1..=10).map(id).collect();
        let event_tags2: HashSet<Uuid> = [id(1), id(2)].into();
        let shared2 = user_tags2.intersection(&event_tags2).count();
        #[allow(clippy::cast_precision_loss)]
        let score2 = (shared2 as f64 / user_tags2.len() as f64) * 100.0;
        assert!((score2 - 20.0).abs() < f64::EPSILON);

        assert!((score - score2).abs() > 1.0);
    }

    #[test]
    fn sort_with_nan_does_not_panic() {
        let mut scored = [(f64::NAN, "a"), (50.0, "b"), (f64::NAN, "c")];
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(scored.len(), 3);
    }
}
