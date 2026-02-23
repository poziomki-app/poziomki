use uuid::Uuid;

use super::events_tags_repo::{find_or_create_event_tag, sync_event_tags};

pub(in crate::api) async fn resolve_event_tag_ids(
    tag_names: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> Vec<Uuid> {
    if let Some(ids) = tag_ids {
        return ids
            .into_iter()
            .filter_map(|s| Uuid::parse_str(&s).ok())
            .collect();
    }

    let mut resolved = Vec::new();
    for raw in tag_names.unwrap_or_default() {
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
    resolved
}

pub(in crate::api) async fn maybe_sync_tags(
    event_id: Uuid,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> std::result::Result<(), crate::error::AppError> {
    if tags.is_some() || tag_ids.is_some() {
        let resolved = resolve_event_tag_ids(tags, tag_ids).await;
        sync_event_tags(event_id, &resolved).await?;
    }
    Ok(())
}
