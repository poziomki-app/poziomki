use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_interactions::EventInteraction;
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profile_tags::ProfileTag;
use crate::db::models::recommendation_feedback::RecommendationFeedback;
use crate::db::models::sessions::Session;
use crate::db::models::tags::Tag;
use crate::db::models::uploads::Upload;
use crate::db::models::user_settings::UserSetting;
use crate::db::schema::event_interactions;
use crate::db::schema::{
    event_attendees, event_tags, events, profile_tags, recommendation_feedback, sessions, tags,
    uploads, user_settings,
};

pub(super) async fn load_user_tags(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let pt_rows = profile_tags::table
        .filter(profile_tags::profile_id.eq(profile_id))
        .load::<ProfileTag>(conn)
        .await?;

    let tag_ids: Vec<Uuid> = pt_rows.iter().map(|pt| pt.tag_id).collect();
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
                "scope": t.scope,
                "category": t.category,
                "emoji": t.emoji,
                "parentId": t.parent_id.map(|id| id.to_string()),
            })
        })
        .collect())
}

pub(super) async fn load_created_events(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let event_rows = events::table
        .filter(events::creator_id.eq(profile_id))
        .load::<Event>(conn)
        .await?;

    let mut result = Vec::with_capacity(event_rows.len());
    for event in &event_rows {
        let event_tag_rows = load_tags_for_event(conn, event.id).await?;
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
    conn: &mut AsyncPgConnection,
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
                "parentId": t.parent_id.map(|id| id.to_string()),
            })
        })
        .collect())
}

pub(super) async fn load_attended_events(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let att_rows = event_attendees::table
        .filter(event_attendees::profile_id.eq(profile_id))
        .load::<EventAttendee>(conn)
        .await?;

    let event_ids: Vec<Uuid> = att_rows.iter().map(|a| a.event_id).collect();
    if event_ids.is_empty() {
        return Ok(vec![]);
    }

    let event_rows = events::table
        .filter(events::id.eq_any(&event_ids))
        .load::<Event>(conn)
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
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let session_rows = sessions::table
        .filter(sessions::user_id.eq(user_id))
        .load::<Session>(conn)
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
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let upload_rows = uploads::table
        .filter(uploads::owner_id.eq(profile_id))
        .filter(uploads::deleted.eq(false))
        .load::<Upload>(conn)
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
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> std::result::Result<Option<serde_json::Value>, crate::error::AppError> {
    let settings = user_settings::table
        .filter(user_settings::user_id.eq(user_id))
        .first::<UserSetting>(conn)
        .await
        .optional()?;

    Ok(settings.map(|s| {
        serde_json::json!({
            "theme": s.theme,
            "language": s.language,
            "notificationsEnabled": s.notifications_enabled,
            "privacyShowProgram": s.privacy_show_program,
            "privacyDiscoverable": s.privacy_discoverable,
        })
    }))
}

pub(super) async fn load_event_interactions(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let interaction_rows = event_interactions::table
        .filter(event_interactions::profile_id.eq(profile_id))
        .load::<EventInteraction>(conn)
        .await?;

    let event_ids: Vec<Uuid> = interaction_rows.iter().map(|row| row.event_id).collect();
    let event_rows = if event_ids.is_empty() {
        vec![]
    } else {
        events::table
            .filter(events::id.eq_any(&event_ids))
            .load::<Event>(conn)
            .await?
    };

    Ok(interaction_rows
        .iter()
        .map(|row| {
            let title = event_rows
                .iter()
                .find(|event| event.id == row.event_id)
                .map(|event| event.title.clone());
            serde_json::json!({
                "eventId": row.event_id.to_string(),
                "title": title,
                "kind": row.kind,
                "createdAt": row.created_at.to_rfc3339(),
                "updatedAt": row.updated_at.to_rfc3339(),
            })
        })
        .collect())
}

pub(super) async fn load_upload_filenames(
    conn: &mut AsyncPgConnection,
    profile_id: uuid::Uuid,
) -> std::result::Result<Vec<String>, crate::error::AppError> {
    let filenames = uploads::table
        .filter(uploads::owner_id.eq(profile_id))
        .filter(uploads::deleted.eq(false))
        .select(uploads::filename)
        .load::<String>(conn)
        .await?;

    Ok(filenames)
}

pub(super) async fn load_recommendation_feedback(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Vec<serde_json::Value>, crate::error::AppError> {
    let rows = recommendation_feedback::table
        .filter(recommendation_feedback::profile_id.eq(profile_id))
        .load::<RecommendationFeedback>(conn)
        .await?;

    let event_ids: Vec<Uuid> = rows.iter().map(|r| r.event_id).collect();
    let event_rows = if event_ids.is_empty() {
        vec![]
    } else {
        events::table
            .filter(events::id.eq_any(&event_ids))
            .load::<Event>(conn)
            .await?
    };

    Ok(rows
        .iter()
        .map(|row| {
            let title = event_rows
                .iter()
                .find(|event| event.id == row.event_id)
                .map(|event| event.title.clone());
            serde_json::json!({
                "eventId": row.event_id.to_string(),
                "title": title,
                "feedback": row.feedback,
                "createdAt": row.created_at.to_rfc3339(),
            })
        })
        .collect())
}
