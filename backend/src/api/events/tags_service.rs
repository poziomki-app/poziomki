use axum::http::HeaderMap;
use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use super::events_service::{validation_error, HandlerError};
use super::events_tags_repo::{
    find_or_create_event_tag_with_conn, load_existing_event_tag_ids, sync_event_tags_with_conn,
};

const MAX_EVENT_TAGS: usize = 15;

/// Parse and deduplicate a list of tag id strings. Returns a handler error
/// response on invalid input. Caller must then validate the parsed ids
/// exist via `validate_event_tag_ids_with_conn` inside a viewer tx.
pub(in crate::api) fn parse_event_tag_ids(
    headers: &HeaderMap,
    ids: Vec<String>,
) -> std::result::Result<Vec<Uuid>, HandlerError> {
    if ids.len() > MAX_EVENT_TAGS {
        return Err(Box::new(validation_error(headers, "Too many tags")));
    }

    let mut parsed = Vec::new();
    for raw in ids {
        let uuid = Uuid::parse_str(&raw)
            .map_err(|_| Box::new(validation_error(headers, "All tagIds must be valid UUIDs")))?;
        parsed.push(uuid);
    }
    parsed.sort_unstable();
    parsed.dedup();
    Ok(parsed)
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
            let matched: std::collections::HashSet<Uuid> = load_existing_event_tag_ids(conn, &ids)
                .await?
                .into_iter()
                .collect();
            if matched.len() != ids.len() {
                return Err(crate::error::AppError::Validation(
                    "All tagIds must reference existing interest tags".to_string(),
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
