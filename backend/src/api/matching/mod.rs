#[path = "assembler.rs"]
mod matching_assembler;
#[path = "repo.rs"]
mod matching_repo;
#[path = "scoring.rs"]
mod matching_scoring;

use std::collections::HashMap;

type Result<T> = crate::error::AppResult<T>;

use crate::api::auth_or_respond;
use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::state::{DataResponse, EventFeedbackRequest, MatchingQuery, SuccessResponse};
use crate::api::{error_response, ErrorSpec};
use crate::db;
use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;
use crate::db::schema::{events, profiles, recommendation_feedback};
use matching_assembler::{
    batch_load_show_program, finalize_recommendations, load_recommendation_data,
};
use matching_repo::MatchingRepository;
use matching_scoring::{
    build_affinity_map, rank_and_take, rank_events_and_take, score_event, score_profile,
};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

fn should_exclude_seen_event(
    event_id: Uuid,
    saved_event_ids: &std::collections::HashSet<Uuid>,
    joined_event_ids: &std::collections::HashSet<Uuid>,
) -> bool {
    saved_event_ids.contains(&event_id) || joined_event_ids.contains(&event_id)
}

pub(super) async fn profiles_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    // IP-keyed limit runs before auth so unauthenticated scrape attempts
    // don't touch the DB auth path.
    if let Err(response) = crate::api::ip_rate_limit::enforce_ip_rate_limit(
        &headers,
        crate::api::ip_rate_limit::IpRateLimitAction::MatchingProfiles,
    )
    .await
    {
        return Ok(*response);
    }

    let repo = MatchingRepository;
    let (_session, user) = auth_or_respond!(headers);

    let limit = query.limit.unwrap_or(10).clamp(1, 50) as usize;

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let data = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let user_ctx = repo
                .load_profile_context(user.id, conn)
                .await
                .map_err(matching_into_diesel)?;

            let candidates = repo
                .load_candidate_profiles(user.id, 200, conn)
                .await
                .map_err(matching_into_diesel)?;

            let candidate_ids: Vec<Uuid> = candidates.iter().map(|c| c.id).collect();
            let all_candidate_tags = repo
                .batch_load_profile_tag_ids(&candidate_ids, conn)
                .await
                .map_err(matching_into_diesel)?;

            let my_program = user_ctx.profile.as_ref().and_then(|p| p.program.clone());

            let candidate_user_ids: Vec<i32> = candidates.iter().map(|c| c.user_id).collect();
            let privacy_map = batch_load_show_program(&candidate_user_ids, conn)
                .await
                .map_err(matching_into_diesel)?;

            let mut scored: Vec<(f64, &Profile)> = candidates
                .iter()
                .map(|candidate| {
                    let candidate_tags = all_candidate_tags
                        .get(&candidate.id)
                        .cloned()
                        .unwrap_or_default();
                    let show_program = privacy_map.get(&candidate.user_id).copied().unwrap_or(true);
                    let score = score_profile(
                        &user_ctx.profile_tag_ids,
                        &candidate_tags,
                        my_program.as_deref(),
                        candidate,
                        show_program,
                    );
                    (score, candidate)
                })
                .collect();

            let top = rank_and_take(&mut scored, limit);

            load_recommendation_data(&top, &repo, conn, &privacy_map)
                .await
                .map_err(matching_into_diesel)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    // imgproxy / thumbhash resolution runs after the tx closes so external
    // storage latency can't hold a DB connection open.
    let data = finalize_recommendations(data).await;

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

fn matching_into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

pub(super) async fn events_recommendations(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MatchingQuery>,
) -> Result<Response> {
    let repo = MatchingRepository;
    let (_session, user) = auth_or_respond!(headers);

    let limit = usize::from(query.limit.unwrap_or(20).clamp(1, 100));
    let candidate_limit =
        i64::try_from(std::cmp::max(limit.saturating_mul(20), 500)).unwrap_or(i64::MAX);

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let now = Utc::now();

    // DB reads + ranking happen inside the viewer tx so Tier-A/B/C policies
    // on events/event_attendees/event_interactions apply. Image URL signing
    // is external I/O and must not hold the DB connection, so it's done
    // after the tx closes.
    let (mut base, score_by_event) = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let user_ctx = repo
                .load_user_context(user.id, conn)
                .await
                .map_err(matching_into_diesel)?;
            let my_profile_id = user_ctx.profile.as_ref().map_or(Uuid::nil(), |p| p.id);

            let future_events = repo
                .load_future_events(now, candidate_limit, conn)
                .await
                .map_err(matching_into_diesel)?
                .into_iter()
                .filter(|event| {
                    !should_exclude_seen_event(
                        event.id,
                        &user_ctx.saved_event_ids,
                        &user_ctx.joined_event_ids,
                    )
                })
                .collect::<Vec<_>>();

            let event_ids: Vec<Uuid> = future_events.iter().map(|e| e.id).collect();
            let all_event_tags = repo
                .batch_load_event_tag_ids(&event_ids, conn)
                .await
                .map_err(matching_into_diesel)?;
            let tag_parent_map = repo
                .load_tag_parent_map(conn)
                .await
                .map_err(matching_into_diesel)?;

            let profile_affinity = build_affinity_map(
                user_ctx
                    .profile_tag_ids
                    .iter()
                    .copied()
                    .map(|tag_id| (tag_id, 1.0)),
                &tag_parent_map,
            );

            let saved_event_ids: Vec<Uuid> = user_ctx.saved_event_ids.iter().copied().collect();
            let joined_event_ids: Vec<Uuid> = user_ctx.joined_event_ids.iter().copied().collect();
            let more_event_ids: Vec<Uuid> = user_ctx.more_event_ids.iter().copied().collect();
            let mut all_history_ids = saved_event_ids.clone();
            all_history_ids.extend(&joined_event_ids);
            all_history_ids.extend(&more_event_ids);
            all_history_ids.sort_unstable();
            all_history_ids.dedup();
            let all_history_tags = repo
                .batch_load_event_tag_ids(&all_history_ids, conn)
                .await
                .map_err(matching_into_diesel)?;
            let mut event_weights: HashMap<Uuid, f64> = HashMap::new();
            for &id in &joined_event_ids {
                event_weights.insert(id, 1.0);
            }
            for &id in &more_event_ids {
                event_weights.entry(id).or_insert(0.7);
            }
            for &id in &saved_event_ids {
                event_weights.entry(id).or_insert(0.5);
            }
            let history_affinity = build_affinity_map(
                event_weights.iter().flat_map(|(id, &weight)| {
                    all_history_tags.get(id).into_iter().flat_map(move |tags| {
                        tags.iter().copied().map(move |tag_id| (tag_id, weight))
                    })
                }),
                &tag_parent_map,
            );

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
                    let mut s = score_event(
                        &profile_affinity,
                        &history_affinity,
                        &user_ctx.interest_categories,
                        &event_tag_ids,
                        event,
                        user_geo,
                        &tag_parent_map,
                    );
                    if user_ctx.less_event_ids.contains(&event.id) {
                        s *= 0.3;
                    }
                    (s, event)
                })
                .collect();

            let top = rank_events_and_take(&mut scored, limit);
            let top_models: Vec<Event> = top.iter().map(|(_, event)| (*event).clone()).collect();
            let base = super::events::build_event_responses_raw(conn, &top_models, &my_profile_id)
                .await
                .map_err(matching_into_diesel)?;
            let score_by_event: HashMap<Uuid, f64> = top
                .iter()
                .map(|(score, event)| (event.id, *score))
                .collect();

            Ok::<_, diesel::result::Error>((base, score_by_event))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    super::events::resolve_event_images_for_responses(&mut base).await;
    let data = base
        .into_iter()
        .map(|mut event| {
            event.score = Uuid::parse_str(&event.id)
                .inspect_err(|e| tracing::warn!("failed to parse event id: {e}"))
                .ok()
                .and_then(|uuid| score_by_event.get(&uuid).copied());
            event
        })
        .collect::<Vec<_>>();

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}

enum FeedbackOutcome {
    NoProfile,
    NotFound,
    Saved,
}

pub(super) async fn event_feedback(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Json(body): Json<EventFeedbackRequest>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if body.feedback != "more" && body.feedback != "less" {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let event_uuid = crate::api::parse_uuid(&event_id, "event")?;

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let feedback = body.feedback.clone();
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let profile_id: Option<Uuid> = profiles::table
                .filter(profiles::user_id.eq(user_id))
                .select(profiles::id)
                .first::<Uuid>(conn)
                .await
                .optional()?;
            let Some(profile_id) = profile_id else {
                return Ok::<FeedbackOutcome, diesel::result::Error>(FeedbackOutcome::NoProfile);
            };

            let event_exists = events::table
                .find(event_uuid)
                .select(events::id)
                .first::<Uuid>(conn)
                .await
                .optional()?;
            if event_exists.is_none() {
                return Ok(FeedbackOutcome::NotFound);
            }

            let now = Utc::now();
            diesel::insert_into(recommendation_feedback::table)
                .values((
                    recommendation_feedback::profile_id.eq(profile_id),
                    recommendation_feedback::event_id.eq(event_uuid),
                    recommendation_feedback::feedback.eq(&feedback),
                    recommendation_feedback::created_at.eq(now),
                ))
                .on_conflict((
                    recommendation_feedback::profile_id,
                    recommendation_feedback::event_id,
                ))
                .do_update()
                .set((
                    recommendation_feedback::feedback.eq(&feedback),
                    recommendation_feedback::created_at.eq(now),
                ))
                .execute(conn)
                .await?;

            Ok(FeedbackOutcome::Saved)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        FeedbackOutcome::NoProfile => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Profile not found. Create a profile first.".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        FeedbackOutcome::NotFound => Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: format!("Event '{event_id}' not found"),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        FeedbackOutcome::Saved => Ok(Json(DataResponse {
            data: SuccessResponse { success: true },
        })
        .into_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::should_exclude_seen_event;
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn seen_events_are_excluded() {
        let seen = Uuid::from_u128(1);
        let saved_event_ids = HashSet::from([seen]);
        let joined_event_ids = HashSet::from([Uuid::from_u128(2)]);

        assert!(should_exclude_seen_event(
            seen,
            &saved_event_ids,
            &joined_event_ids,
        ));
        assert!(should_exclude_seen_event(
            Uuid::from_u128(2),
            &saved_event_ids,
            &joined_event_ids,
        ));
        assert!(!should_exclude_seen_event(
            Uuid::from_u128(3),
            &saved_event_ids,
            &joined_event_ids,
        ));
    }
}
