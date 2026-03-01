#[path = "assembler.rs"]
mod matching_assembler;
#[path = "repo.rs"]
mod matching_repo;
#[path = "scoring.rs"]
mod matching_scoring;

use std::collections::HashMap;

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
use uuid::Uuid;

use super::state::{require_auth_db, DataResponse, MatchingQuery};
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use matching_assembler::build_recommendations_response;
use matching_repo::MatchingRepository;
use matching_scoring::{rank_and_take, rank_events_and_take, score_event, score_profile};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

pub(super) async fn profiles_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let repo = MatchingRepository;
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(10).clamp(1, 50) as usize;

    let mut conn = crate::db::conn().await?;
    let (my_profile, my_tag_ids) = repo.load_user_context(user.id, &mut conn).await?;

    // Fetch candidate profiles (more than limit so we can score and rank)
    let candidates = repo
        .load_candidate_profiles(user.id, 200, &mut conn)
        .await?;

    // Batch-load all candidate tag IDs in one query
    let candidate_ids: Vec<Uuid> = candidates.iter().map(|c| c.id).collect();
    let all_candidate_tags = repo
        .batch_load_profile_tag_ids(&candidate_ids, &mut conn)
        .await?;

    let my_program = my_profile.as_ref().and_then(|p| p.program.clone());

    // Score each candidate
    let mut scored: Vec<(f64, &Profile)> = candidates
        .iter()
        .map(|candidate| {
            let candidate_tags = all_candidate_tags
                .get(&candidate.id)
                .cloned()
                .unwrap_or_default();
            let score = score_profile(
                &my_tag_ids,
                &candidate_tags,
                my_program.as_deref(),
                candidate,
            );
            (score, candidate)
        })
        .collect();

    let top = rank_and_take(&mut scored, limit);

    let data = build_recommendations_response(&top, &repo, &mut conn).await?;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

pub(super) async fn events_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let repo = MatchingRepository;
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100) as usize;

    let mut conn = crate::db::conn().await?;
    let (my_profile, my_tag_ids) = repo.load_user_context(user.id, &mut conn).await?;
    let my_profile_id = my_profile.as_ref().map_or(Uuid::nil(), |p| p.id);

    let now = Utc::now();

    // Fetch future events
    let future_events = repo.load_future_events(now, 100, &mut conn).await?;

    // Batch-load all event tag IDs in one query
    let event_ids: Vec<Uuid> = future_events.iter().map(|e| e.id).collect();
    let all_event_tags = repo.batch_load_event_tag_ids(&event_ids, &mut conn).await?;

    // Resolve user geo query: lat, lng, max radius in km (default 20 km)
    let user_geo = query.lat.zip(query.lng).map(|(lat, lng)| {
        (
            lat,
            lng,
            f64::from(query.radius_m.unwrap_or(20_000)) / 1000.0,
        )
    });

    let mut scored: Vec<(f64, &Event)> = future_events
        .iter()
        .map(|event| {
            let event_tag_ids = all_event_tags.get(&event.id).cloned().unwrap_or_default();
            (
                score_event(&my_tag_ids, &event_tag_ids, event, user_geo),
                event,
            )
        })
        .collect();

    let top = rank_events_and_take(&mut scored, limit);

    let top_models: Vec<Event> = top.iter().map(|(_, event)| (*event).clone()).collect();
    let base =
        super::events::build_event_responses_with_conn(&top_models, &my_profile_id, &mut conn)
            .await?;
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
