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

async fn load_event_tag_ids(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> std::result::Result<HashSet<Uuid>, loco_rs::Error> {
    let links = event_tags::Entity::find()
        .filter(event_tags::Column::EventId.eq(event_id))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Any(e.into()))?;
    Ok(links.iter().map(|l| l.tag_id).collect())
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

    // Score each event by user tag overlap
    let mut scored: Vec<(f64, &events::Model)> = Vec::with_capacity(future_events.len());
    for event in &future_events {
        let event_tag_ids = load_event_tag_ids(&ctx.db, event.id).await?;
        #[allow(clippy::cast_precision_loss)]
        let score = if my_tag_ids.is_empty() {
            0.0
        } else {
            let shared = my_tag_ids.intersection(&event_tag_ids).count();
            (shared as f64 / my_tag_ids.len() as f64) * 100.0
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
#[allow(clippy::float_cmp, clippy::unwrap_used, clippy::useless_vec, clippy::suboptimal_flops)]
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
}
