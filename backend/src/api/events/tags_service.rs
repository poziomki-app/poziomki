use axum::http::HeaderMap;
use axum::response::IntoResponse;
use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use super::events_service::{validation_error, HandlerError};
use super::events_tags_repo::{
    find_or_create_event_tag, find_or_create_event_tag_with_conn, load_existing_event_tag_ids,
    sync_event_tags_with_conn,
};

const MAX_EVENT_TAGS: usize = 15;

pub(in crate::api) async fn resolve_event_tag_ids(
    headers: &HeaderMap,
    tag_names: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> std::result::Result<Vec<Uuid>, HandlerError> {
    if let Some(ids) = tag_ids {
        if ids.len() > MAX_EVENT_TAGS {
            return Err(Box::new(validation_error(headers, "Too many tags")));
        }
        return validate_event_tag_ids(headers, ids).await;
    }

    let names = tag_names.unwrap_or_default();
    if names.len() > MAX_EVENT_TAGS {
        return Err(Box::new(validation_error(headers, "Too many tags")));
    }

    let mut resolved = Vec::new();
    for raw in names {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(id) = find_or_create_event_tag(trimmed).await {
            resolved.push(id);
        }
    }
    resolved.sort_unstable();
    resolved.dedup();
    Ok(resolved)
}

pub(in crate::api) async fn resolve_event_tag_ids_with_conn(
    conn: &mut AsyncPgConnection,
    tag_names: Option<Vec<String>>,
    tag_ids: Option<Vec<Uuid>>,
) -> std::result::Result<Vec<Uuid>, crate::error::AppError> {
    if let Some(mut ids) = tag_ids {
        ids.truncate(MAX_EVENT_TAGS);
        ids.sort_unstable();
        ids.dedup();
        if !ids.is_empty() {
            let matched = load_existing_event_tag_ids(conn, &ids).await?;
            if matched.len() != ids.len() {
                return Err(crate::error::AppError::Validation(
                    "All tagIds must reference existing event tags".to_string(),
                ));
            }
        }
        return Ok(ids);
    }

    let mut resolved = Vec::new();
    for raw in tag_names
        .unwrap_or_default()
        .into_iter()
        .take(MAX_EVENT_TAGS)
    {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        resolved.push(find_or_create_event_tag_with_conn(conn, trimmed).await?);
    }
    resolved.sort_unstable();
    resolved.dedup();
    Ok(resolved)
}

pub(in crate::api) async fn maybe_sync_tags_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<Uuid>>,
) -> std::result::Result<(), crate::error::AppError> {
    if tags.is_some() || tag_ids.is_some() {
        let resolved = resolve_event_tag_ids_with_conn(conn, tags, tag_ids).await?;
        sync_event_tags_with_conn(conn, event_id, &resolved).await?;
    }
    Ok(())
}

async fn validate_event_tag_ids(
    headers: &HeaderMap,
    ids: Vec<String>,
) -> std::result::Result<Vec<Uuid>, HandlerError> {
    let mut parsed = Vec::with_capacity(ids.len());
    for raw in ids {
        let uuid = Uuid::parse_str(&raw)
            .map_err(|_| Box::new(validation_error(headers, "All tagIds must be valid UUIDs")))?;
        parsed.push(uuid);
    }

    parsed.sort_unstable();
    parsed.dedup();

    if parsed.is_empty() {
        return Ok(parsed);
    }

    let mut conn = crate::db::conn()
        .await
        .map_err(|e| Box::new(crate::error::AppError::from(e).into_response()))?;
    let matched = load_existing_event_tag_ids(&mut conn, &parsed)
        .await
        .map_err(|e| Box::new(e.into_response()))?;

    if matched.len() != parsed.len() {
        return Err(Box::new(validation_error(
            headers,
            "All tagIds must reference existing event tags",
        )));
    }

    Ok(parsed)
}
