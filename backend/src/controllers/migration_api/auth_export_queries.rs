use sea_orm::DatabaseConnection;
use sea_orm::QueryFilter;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::_entities::{
    event_attendees, event_tags, events, profile_tags, sessions, tags, uploads, user_settings,
};
#[allow(unused_imports)]
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, IntoActiveModel as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, TransactionTrait as _,
};

pub(super) async fn load_user_tags(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let pt_rows = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = pt_rows.iter().map(|pt| pt.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_rows = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(tag_rows
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "name": t.name,
                "scope": t.scope,
                "category": t.category,
                "emoji": t.emoji,
            })
        })
        .collect())
}

pub(super) async fn load_created_events(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let event_rows = events::Entity::find()
        .filter(events::Column::CreatorId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    let mut result = Vec::with_capacity(event_rows.len());
    for event in &event_rows {
        let event_tag_rows = load_tags_for_event(db, event.id).await?;
        result.push(serde_json::json!({
            "id": event.id.to_string(),
            "title": event.title,
            "description": event.description,
            "coverImage": event.cover_image,
            "location": event.location,
            "startsAt": event.starts_at.to_rfc3339(),
            "endsAt": event.ends_at.map(|e| e.to_rfc3339()),
            "createdAt": event.created_at.to_rfc3339(),
            "tags": event_tag_rows,
        }));
    }
    Ok(result)
}

async fn load_tags_for_event(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let et_rows = event_tags::Entity::find()
        .filter(event_tags::Column::EventId.eq(event_id))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    let tag_ids: Vec<Uuid> = et_rows.iter().map(|et| et.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_rows = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(tag_rows
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "name": t.name,
            })
        })
        .collect())
}

pub(super) async fn load_attended_events(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let att_rows = event_attendees::Entity::find()
        .filter(event_attendees::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    let event_ids: Vec<Uuid> = att_rows.iter().map(|a| a.event_id).collect();
    if event_ids.is_empty() {
        return Ok(vec![]);
    }

    let event_rows = events::Entity::find()
        .filter(events::Column::Id.is_in(event_ids.clone()))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(att_rows
        .iter()
        .filter_map(|a| {
            let event = event_rows.iter().find(|e| e.id == a.event_id)?;
            Some(serde_json::json!({
                "eventId": event.id.to_string(),
                "title": event.title,
                "status": a.status,
                "startsAt": event.starts_at.to_rfc3339(),
            }))
        })
        .collect())
}

pub(super) async fn load_user_sessions(
    db: &DatabaseConnection,
    user_id: i32,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let session_rows = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(session_rows
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "ipAddress": s.ip_address,
                "userAgent": s.user_agent,
                "expiresAt": s.expires_at.to_rfc3339(),
                "createdAt": s.created_at.to_rfc3339(),
            })
        })
        .collect())
}

pub(super) async fn load_user_uploads(
    db: &DatabaseConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, AppError> {
    let upload_rows = uploads::Entity::find()
        .filter(uploads::Column::OwnerId.eq(profile_id))
        .filter(uploads::Column::Deleted.eq(false))
        .all(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(upload_rows
        .iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id.to_string(),
                "filename": u.filename,
                "context": u.context,
                "mimeType": u.mime_type,
                "createdAt": u.created_at.to_rfc3339(),
            })
        })
        .collect())
}

pub(super) async fn load_user_settings(
    db: &DatabaseConnection,
    user_id: i32,
) -> std::result::Result<Option<serde_json::Value>, AppError> {
    let settings = user_settings::Entity::find()
        .filter(user_settings::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::Any(e.into()))?;

    Ok(settings.map(|s| {
        serde_json::json!({
            "theme": s.theme,
            "language": s.language,
            "notificationsEnabled": s.notifications_enabled,
            "privacyShowAge": s.privacy_show_age,
            "privacyShowProgram": s.privacy_show_program,
            "privacyDiscoverable": s.privacy_discoverable,
        })
    }))
}
