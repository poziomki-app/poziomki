use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::api::auth_or_respond;
use super::state::DataResponse;

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");
type Result<T> = crate::error::AppResult<T>;

#[derive(Deserialize)]
pub(super) struct SearchQuery {
    q: String,
    limit: Option<u8>,
    lat: Option<f64>,
    lng: Option<f64>,
    radius_m: Option<u32>,
}

fn with_private_cache_header(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    response
}

fn empty_results_response() -> Response {
    let data = crate::search::SearchResults {
        profiles: vec![],
        events: vec![],
        tags: vec![],
    };
    with_private_cache_header(Json(DataResponse { data }).into_response())
}

fn build_geo_params(query: &SearchQuery) -> Option<crate::search::GeoSearchParams> {
    match (query.lat, query.lng) {
        (Some(lat), Some(lng)) => Some(crate::search::GeoSearchParams {
            lat,
            lng,
            radius_m: query.radius_m.unwrap_or(10_000),
        }),
        _ => None,
    }
}

fn collect_search_image_urls(results: &crate::search::SearchResults) -> Vec<String> {
    let profile_urls = results
        .profiles
        .iter()
        .filter_map(|profile| profile.profile_picture.clone());
    let event_urls = results
        .events
        .iter()
        .filter_map(|event| event.cover_image.clone());
    profile_urls.chain(event_urls).collect()
}

fn apply_resolved_search_image_urls(
    results: &mut crate::search::SearchResults,
    url_map: &HashMap<String, String>,
) {
    for profile in &mut results.profiles {
        if let Some(resolved_url) = profile
            .profile_picture
            .as_ref()
            .and_then(|raw| url_map.get(raw))
            .cloned()
        {
            profile.profile_picture = Some(resolved_url);
        }
    }
    for event in &mut results.events {
        if let Some(resolved_url) = event
            .cover_image
            .as_ref()
            .and_then(|raw| url_map.get(raw))
            .cloned()
        {
            event.cover_image = Some(resolved_url);
        }
    }
}

#[derive(Deserialize)]
pub(super) struct MessageSearchQuery {
    q: String,
    limit: Option<u8>,
}

#[derive(serde::Serialize)]
struct MessageSearchResults {
    room_ids: Vec<String>,
}

pub(super) async fn search_messages(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<MessageSearchQuery>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let limit = usize::from(query.limit.unwrap_or(20).clamp(1, 50));
    let q = query.q.trim().to_string();

    if q.is_empty() {
        return Ok(with_private_cache_header(
            Json(DataResponse {
                data: MessageSearchResults {
                    room_ids: vec![],
                },
            })
            .into_response(),
        ));
    }

    let room_ids = crate::search::search_message_rooms(&q, &user.pid, limit)
        .await
        .map_err(|e| {
            tracing::error!("Message room search failed: {e}");
            crate::error::AppError::Message("Search failed".to_string())
        })?;

    Ok(with_private_cache_header(
        Json(DataResponse {
            data: MessageSearchResults { room_ids },
        })
        .into_response(),
    ))
}

pub(super) async fn search(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<Response> {
    let (_session, _user) = auth_or_respond!(headers);

    let limit = usize::from(query.limit.unwrap_or(10).clamp(1, 50));
    let q = query.q.trim().to_string();

    if q.is_empty() {
        return Ok(empty_results_response());
    }

    let geo = build_geo_params(&query);

    let mut results = crate::search::search_all(&q, limit, geo.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Search query failed: {e}");
            crate::error::AppError::Message("Search failed".to_string())
        })?;

    // Collect all image URLs and resolve in batch.
    let all_urls = collect_search_image_urls(&results);
    let resolved = super::resolve_image_urls(&all_urls).await;
    let url_map: HashMap<String, String> = all_urls.into_iter().zip(resolved).collect();
    apply_resolved_search_image_urls(&mut results, &url_map);

    Ok(with_private_cache_header(
        Json(DataResponse { data: results }).into_response(),
    ))
}
