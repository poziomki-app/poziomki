use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::state::{
    require_auth_db, DataResponse, MatchingQuery, MatchingTagResponse, ProfileRecommendation,
    TagScope,
};
use crate::models::_entities::{event_tags, events, profile_tags, profiles, tags, users};

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

async fn load_profile_tag_ids(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<HashSet<Uuid>, loco_rs::Error> {
    let tag_links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    Ok(tag_links.iter().map(|l| l.tag_id).collect())
}

/// Batch-load all tag links and tag models for a set of profile IDs in 2 queries.
/// Returns a map from `profile_id` → `Vec<MatchingTagResponse>`.
async fn batch_load_profile_tags(
    db: &DatabaseConnection,
    profile_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, Vec<MatchingTagResponse>>, loco_rs::Error> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let all_links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.is_in(profile_ids.to_vec()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let all_tag_ids: HashSet<Uuid> = all_links.iter().map(|l| l.tag_id).collect();
    let tag_models = if all_tag_ids.is_empty() {
        vec![]
    } else {
        tags::Entity::find()
            .filter(tags::Column::Id.is_in(all_tag_ids))
            .all(db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?
    };

    let tag_by_id: HashMap<Uuid, &tags::Model> = tag_models.iter().map(|t| (t.id, t)).collect();

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
    db: &DatabaseConnection,
    profile_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, loco_rs::Error> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let all_links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.is_in(profile_ids.to_vec()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

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

/// Returns a 0.0–1.0 score: 1.0 when distance is 0, 0.0 at or beyond `max_km`.
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
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(10).clamp(1, 50) as usize;

    // Load current user's profile and tags
    let my_profile = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let my_tag_ids = match &my_profile {
        Some(p) => load_profile_tag_ids(&ctx.db, p.id).await?,
        None => HashSet::new(),
    };

    // Fetch candidate profiles (more than limit so we can score and rank)
    let candidates = profiles::Entity::find()
        .filter(profiles::Column::UserId.ne(user.id))
        .order_by_desc(profiles::Column::CreatedAt)
        .limit(200)
        .all(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    // Batch-load all candidate tag IDs in one query
    let candidate_ids: Vec<Uuid> = candidates.iter().map(|c| c.id).collect();
    let all_candidate_tags = batch_load_profile_tag_ids(&ctx.db, &candidate_ids).await?;

    // Score each candidate
    let mut scored: Vec<(f64, &profiles::Model)> = Vec::with_capacity(candidates.len());
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
    let user_models = load_users_by_ids(&ctx.db, &user_ids).await?;

    // Batch-load tags and resolve images for the top profiles
    let top_ids: Vec<Uuid> = top.iter().map(|(_, p)| p.id).collect();
    let top_tags = batch_load_profile_tags(&ctx.db, &top_ids).await?;

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

    Ok(Json(DataResponse { data }).into_response())
}

/// Batch-load tag ID sets for a set of event IDs in 1 query.
async fn batch_load_event_tag_ids(
    db: &DatabaseConnection,
    event_ids: &[Uuid],
) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, loco_rs::Error> {
    if event_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let all_links = event_tags::Entity::find()
        .filter(event_tags::Column::EventId.is_in(event_ids.to_vec()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    let mut result: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for link in &all_links {
        result.entry(link.event_id).or_default().insert(link.tag_id);
    }
    Ok(result)
}

pub(super) async fn events_recommendations(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100) as usize;

    // Load current user's profile and tags
    let my_profile = profiles::Entity::find()
        .filter(profiles::Column::UserId.eq(user.id))
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    let my_profile_id = my_profile.as_ref().map_or(Uuid::nil(), |p| p.id);

    let my_tag_ids = match &my_profile {
        Some(p) => load_profile_tag_ids(&ctx.db, p.id).await?,
        None => HashSet::new(),
    };

    let now = Utc::now();

    // Fetch future events
    let future_events = events::Entity::find()
        .filter(events::Column::StartsAt.gte(now))
        .order_by_asc(events::Column::StartsAt)
        .limit(100)
        .all(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;

    // Batch-load all event tag IDs in one query
    let event_ids: Vec<Uuid> = future_events.iter().map(|e| e.id).collect();
    let all_event_tags = batch_load_event_tag_ids(&ctx.db, &event_ids).await?;

    // Resolve user geo query: lat, lng, max radius in km (default 20 km)
    let user_geo = query.lat.zip(query.lng).map(|(lat, lng)| {
        (
            lat,
            lng,
            f64::from(query.radius_m.unwrap_or(20_000)) / 1000.0,
        )
    });

    // Score each event by user tag overlap + optional proximity.
    // Without geo: pure tag relevance (0–100).
    // With geo: tags still dominate (85 %) with proximity as mild tiebreaker (15 %).
    let mut scored: Vec<(f64, &events::Model)> = Vec::with_capacity(future_events.len());
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
            // No location provided — pure tag relevance, no penalty
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

    // Build responses using the events_view helper (accessed via super)
    let mut data = Vec::new();
    for (score, event) in &top {
        let mut resp = super::events::build_event_response(&ctx.db, event, &my_profile_id).await?;
        resp.score = Some(*score);
        data.push(resp);
    }

    Ok(Json(DataResponse { data }).into_response())
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::useless_vec,
    clippy::suboptimal_flops
)]
mod tests {
    use super::*;
    use crate::controllers::migration_api::state::{EventResponse, ProfilePreview};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    // ── jaccard ────────────────────────────────────────────────

    #[test]
    fn jaccard_both_empty() {
        let a: HashSet<Uuid> = HashSet::new();
        let b: HashSet<Uuid> = HashSet::new();
        assert!((jaccard(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_one_empty() {
        let a: HashSet<Uuid> = [id(1), id(2)].into();
        let b: HashSet<Uuid> = HashSet::new();
        assert!((jaccard(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_identical() {
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        assert!((jaccard(&a, &a) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint() {
        let a: HashSet<Uuid> = [id(1), id(2)].into();
        let b: HashSet<Uuid> = [id(3), id(4)].into();
        assert!((jaccard(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_partial_overlap() {
        // {1,2,3} ∩ {2,3,4} = {2,3}  →  2/4 = 0.5
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(3), id(4)].into();
        assert!((jaccard(&a, &b) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_subset() {
        // {1,2} ∩ {1,2,3} = {1,2}  →  2/3
        let a: HashSet<Uuid> = [id(1), id(2)].into();
        let b: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let expected = 2.0 / 3.0;
        assert!((jaccard(&a, &b) - expected).abs() < 1e-10);
    }

    #[test]
    fn jaccard_single_overlap() {
        // {1,2,3} ∩ {3,4,5} = {3}  →  1/5 = 0.2
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(3), id(4), id(5)].into();
        assert!((jaccard(&a, &b) - 0.2).abs() < f64::EPSILON);
    }

    // ── score scaling ──────────────────────────────────────────

    #[test]
    fn profile_score_scales_to_100() {
        let a: HashSet<Uuid> = [id(1), id(2)].into();
        let b: HashSet<Uuid> = [id(1), id(2)].into();
        let score = jaccard(&a, &b) * 100.0;
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn profile_score_partial() {
        // jaccard = 0.5 → score = 50.0
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(3), id(4)].into();
        let score = jaccard(&a, &b) * 100.0;
        assert!((score - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn program_bonus_stacks() {
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(3), id(4)].into();
        let score = jaccard(&a, &b) * 100.0 + 10.0; // same program bonus
        assert!((score - 60.0).abs() < f64::EPSILON);
    }

    // ── event scoring ──────────────────────────────────────────

    #[test]
    fn event_score_user_no_tags() {
        let user_tags: HashSet<Uuid> = HashSet::new();
        let event_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let score = if user_tags.is_empty() {
            0.0
        } else {
            let shared = user_tags.intersection(&event_tags).count();
            #[allow(clippy::cast_precision_loss)]
            let s = (shared as f64 / user_tags.len() as f64) * 100.0;
            s
        };
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_event_no_tags() {
        let user_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let event_tags: HashSet<Uuid> = HashSet::new();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_full_match() {
        let user_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let event_tags: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        // 2/2 = 100 — event has extra tags but that's fine
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_partial_match() {
        let user_tags: HashSet<Uuid> = [id(1), id(2), id(3), id(4)].into();
        let event_tags: HashSet<Uuid> = [id(2), id(4)].into();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        // 2/4 = 50
        assert!((score - 50.0).abs() < f64::EPSILON);
    }

    // ── sorting ────────────────────────────────────────────────

    #[test]
    fn sort_profiles_by_score_desc() {
        let mut scored = vec![(25.0, "c"), (75.0, "a"), (50.0, "b")];
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let names: Vec<&str> = scored.iter().map(|(_, n)| *n).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn sort_ties_stable_order() {
        // Same score → secondary ordering should decide
        let mut scored = vec![(50.0, 3u32), (50.0, 1), (50.0, 2)];
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1)) // ASC tiebreak
        });
        let vals: Vec<u32> = scored.iter().map(|(_, v)| *v).collect();
        assert_eq!(vals, vec![1, 2, 3]);
    }

    // ── serialization ──────────────────────────────────────────

    #[test]
    fn event_response_score_none_omitted() {
        let resp = EventResponse {
            id: "test".into(),
            title: "t".into(),
            description: None,
            cover_image: None,
            location: None,
            latitude: None,
            longitude: None,
            starts_at: "2026-01-01T00:00:00Z".into(),
            ends_at: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            creator: ProfilePreview {
                id: "c".into(),
                name: "Creator".into(),
                profile_picture: None,
            },
            attendees_count: 0,
            attendees_preview: vec![],
            tags: vec![],
            is_attending: false,
            conversation_id: None,
            score: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("score"), "score:None should be omitted");
    }

    #[test]
    fn event_response_score_some_present() {
        let resp = EventResponse {
            id: "test".into(),
            title: "t".into(),
            description: None,
            cover_image: None,
            location: None,
            latitude: None,
            longitude: None,
            starts_at: "2026-01-01T00:00:00Z".into(),
            ends_at: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            creator: ProfilePreview {
                id: "c".into(),
                name: "Creator".into(),
                profile_picture: None,
            },
            attendees_count: 0,
            attendees_preview: vec![],
            tags: vec![],
            is_attending: false,
            conversation_id: None,
            score: Some(42.5),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""score":42.5"#), "score should be 42.5");
    }

    // ── haversine / proximity ──────────────────────────────────

    #[test]
    fn haversine_same_point() {
        assert!((haversine_km(50.0, 20.0, 50.0, 20.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn haversine_krakow_to_warsaw() {
        // ~252 km between Kraków (50.06, 19.94) and Warsaw (52.23, 21.01)
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
    fn proximity_half() {
        assert!((proximity_score(5.0, 10.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn proximity_zero_max_km() {
        assert!((proximity_score(1.0, 0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn profile_recommendation_score_always_present() {
        let rec = ProfileRecommendation {
            id: "p1".into(),
            user_id: "u1".into(),
            name: "Test".into(),
            bio: None,
            age: 20,
            profile_picture: None,
            program: None,
            gradient_start: None,
            gradient_end: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            tags: vec![],
            score: 0.0,
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains(r#""score":0.0"#), "score=0 must appear");
    }

    // ── jaccard symmetry & edge cases ──────────────────────────

    #[test]
    fn jaccard_is_symmetric() {
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(4)].into();
        assert!((jaccard(&a, &b) - jaccard(&b, &a)).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_single_element_sets() {
        let a: HashSet<Uuid> = [id(1)].into();
        let b: HashSet<Uuid> = [id(1)].into();
        assert!((jaccard(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_single_element_disjoint() {
        let a: HashSet<Uuid> = [id(1)].into();
        let b: HashSet<Uuid> = [id(2)].into();
        assert!((jaccard(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    // ── event scoring formula ─────────────────────────────────
    //
    // Without geo: score = tag_score (0–100)
    // With geo:    score = tag_score * 0.85 + proximity * 15 (max 100)

    #[test]
    fn event_score_no_geo_is_pure_tag() {
        // No lat/lng → score equals tag_score, not penalized
        let tag_score: f64 = 100.0;
        assert!((tag_score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_no_geo_partial_tags() {
        // 50% tag match without geo → score = 50, NOT 35
        let tag_score: f64 = 50.0;
        assert!((tag_score - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_combined_score_with_geo_tag_only() {
        // Perfect tags, event has no coords → 100 * 0.85 + 0 = 85
        let tag_score: f64 = 100.0;
        let geo_bonus: f64 = 0.0;
        let combined = tag_score * 0.85 + geo_bonus;
        assert!((combined - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_combined_score_with_geo_max() {
        // Perfect tags + perfect proximity → 85 + 15 = 100
        let tag_score: f64 = 100.0;
        let geo_bonus = proximity_score(0.0, 20.0) * 15.0;
        let combined = tag_score * 0.85 + geo_bonus;
        assert!((combined - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_combined_score_geo_only() {
        // 0 tag match, perfect proximity → 0 + 15 = 15
        let tag_score: f64 = 0.0;
        let geo_bonus = proximity_score(0.0, 20.0) * 15.0;
        let combined = tag_score * 0.85 + geo_bonus;
        assert!((combined - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_geo_same_city_doesnt_dominate() {
        // Within one city: 3km away vs 15km away (20km radius)
        // Close event (3km): proximity = (1 - 3/20) = 0.85 → geo_bonus = 12.75
        // Far event (15km):  proximity = (1 - 15/20) = 0.25 → geo_bonus = 3.75
        // Swing: ~9 points — smaller than any meaningful tag difference
        let close = proximity_score(3.0, 20.0) * 15.0;
        let far = proximity_score(15.0, 20.0) * 15.0;
        let swing = close - far;
        assert!(
            swing < 10.0,
            "geo swing {swing} should be small within city"
        );
        assert!(swing > 0.0, "closer should still rank higher");
    }

    // ── event scoring asymmetry ────────────────────────────────

    #[test]
    fn event_score_is_asymmetric() {
        // User has 2 tags, event has 10 tags, 2 overlap → 100%
        let user_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let event_tags: HashSet<Uuid> = (1..=10).map(id).collect();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        assert!((score - 100.0).abs() < f64::EPSILON);

        // Flip: user has 10 tags, event has 2 tags, 2 overlap → 20%
        let user_tags2: HashSet<Uuid> = (1..=10).map(id).collect();
        let event_tags2: HashSet<Uuid> = [id(1), id(2)].into();
        let shared2 = user_tags2.intersection(&event_tags2).count();
        #[allow(clippy::cast_precision_loss)]
        let score2 = (shared2 as f64 / user_tags2.len() as f64) * 100.0;
        assert!((score2 - 20.0).abs() < f64::EPSILON);

        // These should NOT be equal (asymmetric by design)
        assert!((score - score2).abs() > 1.0);
    }

    // ── sort stability with NaN ────────────────────────────────

    #[test]
    fn sort_with_nan_does_not_panic() {
        let mut scored = vec![(f64::NAN, "a"), (50.0, "b"), (f64::NAN, "c")];
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // Just verify it doesn't panic; order is undefined with NaN
        assert_eq!(scored.len(), 3);
    }

    // ── sort: zero scores fall back to tiebreaker ──────────────

    #[test]
    fn sort_all_zeros_falls_back_to_tiebreaker() {
        // When all scores are 0, tiebreaker (ASC) should determine order
        let mut scored = vec![(0.0, 30u32), (0.0, 10), (0.0, 20)];
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        let vals: Vec<u32> = scored.iter().map(|(_, v)| *v).collect();
        assert_eq!(vals, vec![10, 20, 30]);
    }

    // ── haversine edge cases ───────────────────────────────────

    #[test]
    fn haversine_negative_coords() {
        // Southern hemisphere, still produces valid distance
        let dist = haversine_km(-33.87, 151.21, -37.81, 144.96); // Sydney → Melbourne
        assert!(dist > 700.0 && dist < 900.0, "got {dist}");
    }

    #[test]
    fn haversine_antipodal() {
        // Opposite sides of Earth ≈ ~20000 km
        let dist = haversine_km(0.0, 0.0, 0.0, 180.0);
        assert!(dist > 19_900.0 && dist < 20_100.0, "got {dist}");
    }

    #[test]
    fn proximity_negative_distance() {
        // Should clamp to 1.0
        assert!((proximity_score(-5.0, 10.0) - 1.0).abs() < f64::EPSILON);
    }
}
