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
/// Returns a map from profile_id → Vec<MatchingTagResponse>.
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
            .filter(tags::Column::Id.is_in(all_tag_ids.into_iter().collect::<Vec<_>>()))
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
