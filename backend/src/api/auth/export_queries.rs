use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profile_tags::ProfileTag;
use crate::db::models::sessions::Session;
use crate::db::models::tags::Tag;
use crate::db::models::uploads::Upload;
use crate::db::models::user_settings::UserSetting;
use crate::db::schema::{
    event_attendees, event_tags, events, profile_tags, sessions, tags, uploads, user_settings,
};

pub(super) async fn load_user_tags(
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let pt_rows = profile_tags::table
        .filter(profile_tags::profile_id.eq(profile_id))
        .load::<ProfileTag>(&mut conn)
        .await?;

    let tag_ids: Vec<Uuid> = pt_rows.iter().map(|pt| pt.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_rows = tags::table
        .filter(tags::id.eq_any(&tag_ids))
        .load::<Tag>(&mut conn)
        .await?;

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
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let event_rows = events::table
        .filter(events::creator_id.eq(profile_id))
        .load::<Event>(&mut conn)
        .await?;

    let mut result = Vec::with_capacity(event_rows.len());
    for event in &event_rows {
        let event_tag_rows = load_tags_for_event(&mut conn, event.id).await?;
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
    conn: &mut diesel_async::AsyncPgConnection,
    event_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let et_rows = event_tags::table
        .filter(event_tags::event_id.eq(event_id))
        .load::<EventTag>(conn)
        .await?;

    let tag_ids: Vec<Uuid> = et_rows.iter().map(|et| et.tag_id).collect();
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let tag_rows = tags::table
        .filter(tags::id.eq_any(&tag_ids))
        .load::<Tag>(conn)
        .await?;

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
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let att_rows = event_attendees::table
        .filter(event_attendees::profile_id.eq(profile_id))
        .load::<EventAttendee>(&mut conn)
        .await?;

    let event_ids: Vec<Uuid> = att_rows.iter().map(|a| a.event_id).collect();
    if event_ids.is_empty() {
        return Ok(vec![]);
    }

    let event_rows = events::table
        .filter(events::id.eq_any(&event_ids))
        .load::<Event>(&mut conn)
        .await?;

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
    user_id: i32,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let session_rows = sessions::table
        .filter(sessions::user_id.eq(user_id))
        .load::<Session>(&mut conn)
        .await?;

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
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let upload_rows = uploads::table
        .filter(uploads::owner_id.eq(profile_id))
        .filter(uploads::deleted.eq(false))
        .load::<Upload>(&mut conn)
        .await?;

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
    user_id: i32,
) -> std::result::Result<Option<serde_json::Value>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let settings = user_settings::table
        .filter(user_settings::user_id.eq(user_id))
        .first::<UserSetting>(&mut conn)
        .await
        .optional()?;

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
