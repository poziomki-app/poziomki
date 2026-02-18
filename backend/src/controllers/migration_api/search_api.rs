use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use serde::Deserialize;

use super::state::{require_auth_db, DataResponse};

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

#[derive(Deserialize)]
pub(super) struct SearchQuery {
    q: String,
    limit: Option<u8>,
    lat: Option<f64>,
    lng: Option<f64>,
    radius_m: Option<u32>,
}

pub(super) async fn search(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let limit = usize::from(query.limit.unwrap_or(10).clamp(1, 50));
    let q = query.q.trim().to_string();

    if q.is_empty() {
        let data = crate::search::SearchResults {
            profiles: vec![],
            events: vec![],
            tags: vec![],
            degrees: vec![],
        };
        let mut response = Json(DataResponse { data }).into_response();
        response
            .headers_mut()
            .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
        return Ok(response);
    }

    let geo = match (query.lat, query.lng) {
        (Some(lat), Some(lng)) => Some(crate::search::GeoSearchParams {
            lat,
            lng,
            radius_m: query.radius_m.unwrap_or(10_000),
        }),
        _ => None,
    };

    let client = crate::search::create_client().map_err(|e| {
        tracing::error!("Failed to create Meilisearch client: {e}");
        loco_rs::Error::Message("Search service unavailable".to_string())
    })?;

    let mut results = crate::search::search_all(&client, &q, limit, geo.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Search query failed: {e}");
            loco_rs::Error::Message("Search failed".to_string())
        })?;

    // Collect all image URLs and resolve in batch
    let mut all_urls: Vec<String> = Vec::new();
    for profile in &results.profiles {
        if let Some(pic) = &profile.profile_picture {
            all_urls.push(pic.clone());
        }
    }
    for event in &results.events {
        if let Some(img) = &event.cover_image {
            all_urls.push(img.clone());
        }
    }
    let resolved = super::resolve_image_urls(&all_urls).await;
    let url_map: std::collections::HashMap<String, String> =
        all_urls.into_iter().zip(resolved).collect();

    for profile in &mut results.profiles {
        if let Some(pic) = &profile.profile_picture {
            if let Some(resolved_url) = url_map.get(pic) {
                profile.profile_picture = Some(resolved_url.clone());
            }
        }
    }
    for event in &mut results.events {
        if let Some(img) = &event.cover_image {
            if let Some(resolved_url) = url_map.get(img) {
                event.cover_image = Some(resolved_url.clone());
            }
        }
    }

    let mut response = Json(DataResponse { data: results }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    Ok(response)
}
