use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::state::{TagResponse, TagScope};
use crate::db::models::profile_tags::ProfileTag;
use crate::db::schema::profile_tags;

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

pub(in crate::api) async fn load_profile_tags(
    profile_id: Uuid,
) -> std::result::Result<Vec<TagResponse>, crate::error::AppError> {
    use crate::db::models::tags::Tag;
    use crate::db::schema::tags;

    let mut conn = crate::db::conn().await?;

    let tag_links = profile_tags::table
        .filter(profile_tags::profile_id.eq(profile_id))
        .load::<ProfileTag>(&mut conn)
        .await?;

    let tag_ids: Vec<Uuid> = tag_links.iter().map(|link| link.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_models = tags::table
        .filter(tags::id.eq_any(&tag_ids))
        .load::<Tag>(&mut conn)
        .await?;

    Ok(tag_models
        .iter()
        .map(|t| TagResponse {
            id: t.id.to_string(),
            name: t.name.clone(),
            scope: scope_from_str(&t.scope),
            category: t.category.clone(),
            emoji: t.emoji.clone(),
            parent_id: t.parent_id.map(|id| id.to_string()),
            onboarding_order: t.onboarding_order.clone(),
        })
        .collect())
}

pub(in crate::api) async fn sync_profile_tags(
    profile_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    diesel::delete(profile_tags::table.filter(profile_tags::profile_id.eq(profile_id)))
        .execute(&mut conn)
        .await?;

    let new_tags: Vec<ProfileTag> = tag_ids
        .iter()
        .map(|tag_id| ProfileTag {
            profile_id,
            tag_id: *tag_id,
        })
        .collect();

    if !new_tags.is_empty() {
        diesel::insert_into(profile_tags::table)
            .values(&new_tags)
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}
