use chrono::Utc;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use crate::models::_entities::{event_attendees, tags};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};

async fn find_or_create_event_tag(db: &DatabaseConnection, name: String) -> Option<Uuid> {
    if let Ok(Some(tag)) = tags::Entity::find()
        .filter(tags::Column::Scope.eq("event"))
        .filter(tags::Column::Name.eq(&name))
        .one(db)
        .await
    {
        return Some(tag.id);
    }

    let new_id = Uuid::new_v4();
    let now = Utc::now();
    let tag = tags::ActiveModel {
        id: ActiveValue::Set(new_id),
        name: ActiveValue::Set(name),
        scope: ActiveValue::Set("event".to_string()),
        category: ActiveValue::Set(None),
        emoji: ActiveValue::Set(None),
        onboarding_order: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };
    tag.insert(db).await.ok().map(|_| new_id)
}

pub(in crate::controllers::migration_api) async fn resolve_event_tag_ids(
    db: &DatabaseConnection,
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
        if let Some(id) = find_or_create_event_tag(db, trimmed).await {
            resolved.push(id);
        }
    }
    resolved.sort_unstable();
    resolved.dedup();
    resolved
}

pub(in crate::controllers::migration_api) async fn sync_event_tags(
    db: &DatabaseConnection,
    event_id: Uuid,
    tag_ids: &[Uuid],
) -> std::result::Result<(), crate::error::AppError> {
    use crate::models::_entities::event_tags;
    event_tags::Entity::delete_many()
        .filter(event_tags::Column::EventId.eq(event_id))
        .exec(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    for tag_id in tag_ids {
        let link = event_tags::ActiveModel {
            event_id: ActiveValue::Set(event_id),
            tag_id: ActiveValue::Set(*tag_id),
        };
        link.insert(db)
            .await
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
    }
    Ok(())
}

pub(in crate::controllers::migration_api) async fn maybe_sync_tags(
    db: &DatabaseConnection,
    event_id: Uuid,
    tags: Option<Vec<String>>,
    tag_ids: Option<Vec<String>>,
) -> std::result::Result<(), crate::error::AppError> {
    if tags.is_some() || tag_ids.is_some() {
        let resolved = resolve_event_tag_ids(db, tags, tag_ids).await;
        sync_event_tags(db, event_id, &resolved).await?;
    }
    Ok(())
}

pub(in crate::controllers::migration_api) async fn upsert_attendee(
    db: &DatabaseConnection,
    event_uuid: Uuid,
    profile_id: Uuid,
    status: &str,
) -> std::result::Result<(), crate::error::AppError> {
    event_attendees::Entity::delete_many()
        .filter(event_attendees::Column::EventId.eq(event_uuid))
        .filter(event_attendees::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let attendee = event_attendees::ActiveModel {
        event_id: ActiveValue::Set(event_uuid),
        profile_id: ActiveValue::Set(profile_id),
        status: ActiveValue::Set(status.to_string()),
    };
    attendee
        .insert(db)
        .await
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    Ok(())
}
