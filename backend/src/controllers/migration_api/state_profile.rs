use super::{
    state_types::{
        FullProfileResponse, MigrationState, ProfilePreview, ProfileRecord, ProfileResponse,
        TagRecord, TagResponse,
    },
    ErrorSpec,
};

pub(in crate::controllers::migration_api) fn to_tag_response(tag: &TagRecord) -> TagResponse {
    TagResponse {
        id: tag.id.clone(),
        name: tag.name.clone(),
        scope: tag.scope,
        category: tag.category.clone(),
        emoji: tag.emoji.clone(),
        onboarding_order: tag.onboarding_order.clone(),
    }
}

pub(in crate::controllers::migration_api) fn to_profile_preview(
    profile: &ProfileRecord,
) -> ProfilePreview {
    ProfilePreview {
        id: profile.id.clone(),
        name: profile.name.clone(),
        profile_picture: profile.profile_picture.clone(),
    }
}

pub(in crate::controllers::migration_api) fn to_profile_response(
    profile: &ProfileRecord,
) -> ProfileResponse {
    ProfileResponse {
        id: profile.id.clone(),
        user_id: profile.user_id.clone(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: profile.age,
        profile_picture: profile.profile_picture.clone(),
        images: profile.images.clone(),
        program: profile.program.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

pub(in crate::controllers::migration_api) fn to_full_profile_response(
    state: &MigrationState,
    profile: &ProfileRecord,
) -> FullProfileResponse {
    let tags = profile
        .tag_ids
        .iter()
        .filter_map(|id| state.tags.get(id))
        .map(to_tag_response)
        .collect::<Vec<_>>();

    FullProfileResponse {
        id: profile.id.clone(),
        user_id: profile.user_id.clone(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: profile.age,
        profile_picture: profile.profile_picture.clone(),
        images: profile.images.clone(),
        program: profile.program.clone(),
        tags,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

pub(in crate::controllers::migration_api) fn normalized_tag_ids(
    state: &MigrationState,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> Vec<String> {
    let raw = tags.unwrap_or_else(|| tag_ids.unwrap_or_default());

    raw.into_iter()
        .filter(|id| state.tags.contains_key(id))
        .collect::<Vec<_>>()
}

pub(in crate::controllers::migration_api) fn validate_profile_name(
    name: &str,
) -> std::result::Result<(), &'static str> {
    if name.trim().is_empty() {
        Err("Name is required")
    } else if name.chars().count() > 100 {
        Err("Name must be at most 100 characters")
    } else if name.contains("http://") || name.contains("https://") || name.contains("www.") {
        Err("Imie nie moze zawierac linkow ani adresow email")
    } else {
        Ok(())
    }
}

pub(in crate::controllers::migration_api) fn validate_profile_age(
    age: u8,
) -> std::result::Result<(), &'static str> {
    if !(15..=67).contains(&age) {
        return Err("Age must be between 15 and 67");
    }
    Ok(())
}

pub(in crate::controllers::migration_api) fn bounded_limit(limit: Option<u8>) -> usize {
    usize::from(limit.unwrap_or(20).clamp(1, 100))
}

pub(in crate::controllers::migration_api) fn bounded_matching_limit(limit: Option<u8>) -> usize {
    usize::from(limit.unwrap_or(10).clamp(1, 50))
}

pub(in crate::controllers::migration_api) fn build_date_range_error() -> ErrorSpec {
    ErrorSpec {
        error: "Event end time must be after start time".to_string(),
        code: "INVALID_DATE_RANGE",
        details: None,
    }
}
