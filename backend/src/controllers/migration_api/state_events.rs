use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    state_profile::build_date_range_error,
    state_types::{MigrationState, TagRecord, TagScope},
    ErrorSpec,
};

pub(in crate::controllers::migration_api) fn parse_timestamp(
    value: &str,
) -> std::result::Result<DateTime<Utc>, &'static str> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| "Invalid date-time format")
}

pub(in crate::controllers::migration_api) fn validate_event_title(
    value: &str,
) -> std::result::Result<String, &'static str> {
    let normalized = value.trim();
    let length = normalized.chars().count();
    if length == 0 {
        Err("Title is required")
    } else if length > 200 {
        Err("Title must be at most 200 characters")
    } else {
        Ok(normalized.to_string())
    }
}

pub(in crate::controllers::migration_api) fn validate_event_description(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 2_000) {
        Err("Description must be at most 2000 characters")
    } else {
        Ok(())
    }
}

pub(in crate::controllers::migration_api) fn validate_event_location(
    value: Option<&String>,
) -> std::result::Result<(), &'static str> {
    if value.is_some_and(|text| text.chars().count() > 500) {
        Err("Location must be at most 500 characters")
    } else {
        Ok(())
    }
}

pub(in crate::controllers::migration_api) fn ensure_valid_event_range(
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
) -> std::result::Result<(), ErrorSpec> {
    if ends_at.is_some_and(|end| end <= starts_at) {
        Err(build_date_range_error())
    } else {
        Ok(())
    }
}

fn find_event_tag_id_by_name(state: &MigrationState, value: &str) -> Option<String> {
    state
        .tags
        .values()
        .find(|tag| tag.scope == TagScope::Event && tag.name.eq_ignore_ascii_case(value))
        .map(|tag| tag.id.clone())
}

fn insert_event_tag(state: &mut MigrationState, name: &str) -> String {
    let tag = TagRecord {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        scope: TagScope::Event,
        category: None,
        emoji: None,
        onboarding_order: None,
    };
    let id = tag.id.clone();
    state.tags.insert(id.clone(), tag);
    id
}

fn resolve_single_event_tag_id(state: &mut MigrationState, raw_value: &str) -> Option<String> {
    let value = raw_value.trim();
    if value.is_empty() {
        return None;
    }
    if state.tags.contains_key(value) {
        return Some(value.to_string());
    }
    find_event_tag_id_by_name(state, value).or_else(|| Some(insert_event_tag(state, value)))
}

pub(in crate::controllers::migration_api) fn resolve_event_tag_ids(
    state: &mut MigrationState,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> Vec<String> {
    if let Some(ids) = tag_ids {
        return ids
            .into_iter()
            .filter(|id| state.tags.contains_key(id))
            .collect::<Vec<_>>();
    }

    let mut resolved = Vec::new();
    for raw_value in tags.unwrap_or_default() {
        if let Some(tag_id) = resolve_single_event_tag_id(state, &raw_value) {
            resolved.push(tag_id);
        }
    }

    resolved.sort_unstable();
    resolved.dedup();
    resolved
}
