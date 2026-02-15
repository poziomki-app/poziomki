use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use serde::Deserialize;

use super::state::{require_auth_db, DataResponse};

#[derive(Deserialize)]
pub(super) struct SearchQuery {
    q: String,
    limit: Option<u8>,
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

    let client = crate::search::create_client().map_err(|e| {
        tracing::error!("Failed to create Meilisearch client: {e}");
        loco_rs::Error::Message("Search service unavailable".to_string())
    })?;

    let mut results = crate::search::search_all(&client, &query.q, limit)
        .await
        .map_err(|e| {
            tracing::error!("Search query failed: {e}");
            loco_rs::Error::Message("Search failed".to_string())
        })?;

    for profile in &mut results.profiles {
        if let Some(pic) = &profile.profile_picture {
            profile.profile_picture = Some(super::resolve_image_url(pic).await);
        }
    }
    for event in &mut results.events {
        if let Some(img) = &event.cover_image {
            event.cover_image = Some(super::resolve_image_url(img).await);
        }
    }

    Ok(Json(DataResponse { data: results }).into_response())
}
