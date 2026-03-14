use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::conversation_members::NewConversationMember;
use crate::db::models::conversations::{Conversation, NewConversation};
use crate::db::schema::{conversation_members, conversations, events, profiles, users};

/// Resolve or create a DM conversation between two users.
/// Uses canonical pair (lower id, higher id) for uniqueness.
#[allow(clippy::similar_names)]
pub async fn resolve_or_create_dm(
    first_user_id: i32,
    second_user_id: i32,
) -> Result<Conversation, crate::error::AppError> {
    let (low, high) = if first_user_id < second_user_id {
        (first_user_id, second_user_id)
    } else {
        (second_user_id, first_user_id)
    };

    let mut conn = crate::db::conn().await?;

    // Try to find existing DM
    let existing = conversations::table
        .filter(conversations::kind.eq("dm"))
        .filter(conversations::user_low_id.eq(low))
        .filter(conversations::user_high_id.eq(high))
        .first::<Conversation>(&mut conn)
        .await
        .optional()?;

    if let Some(found) = existing {
        return Ok(found);
    }

    // Create new DM conversation + members
    let now = Utc::now();
    let new_conv = NewConversation {
        id: Uuid::new_v4(),
        kind: "dm".to_string(),
        title: None,
        event_id: None,
        user_low_id: Some(low),
        user_high_id: Some(high),
        created_at: now,
        updated_at: now,
    };

    let inserted = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .on_conflict_do_nothing()
        .get_result::<Conversation>(&mut conn)
        .await
        .optional()?;

    // Handle race condition: another request may have created it
    let created = match inserted {
        Some(c) => c,
        None => {
            conversations::table
                .filter(conversations::kind.eq("dm"))
                .filter(conversations::user_low_id.eq(low))
                .filter(conversations::user_high_id.eq(high))
                .first::<Conversation>(&mut conn)
                .await?
        }
    };

    // Ensure both users are members
    let members = vec![
        NewConversationMember {
            conversation_id: created.id,
            user_id: low,
            joined_at: now,
        },
        NewConversationMember {
            conversation_id: created.id,
            user_id: high,
            joined_at: now,
        },
    ];
    diesel::insert_into(conversation_members::table)
        .values(&members)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(created)
}

/// Resolve or create an event conversation.
pub async fn resolve_or_create_event_conversation(
    event_id: Uuid,
    event_title: &str,
    creator_user_id: i32,
) -> Result<Conversation, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let existing = conversations::table
        .filter(conversations::kind.eq("event"))
        .filter(conversations::event_id.eq(event_id))
        .first::<Conversation>(&mut conn)
        .await
        .optional()?;

    if let Some(found) = existing {
        return Ok(found);
    }

    let now = Utc::now();
    let new_conv = NewConversation {
        id: Uuid::new_v4(),
        kind: "event".to_string(),
        title: Some(event_title.to_string()),
        event_id: Some(event_id),
        user_low_id: None,
        user_high_id: None,
        created_at: now,
        updated_at: now,
    };

    let inserted = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .on_conflict_do_nothing()
        .get_result::<Conversation>(&mut conn)
        .await
        .optional()?;

    let created = match inserted {
        Some(c) => c,
        None => {
            conversations::table
                .filter(conversations::kind.eq("event"))
                .filter(conversations::event_id.eq(event_id))
                .first::<Conversation>(&mut conn)
                .await?
        }
    };

    // Add creator as first member
    diesel::insert_into(conversation_members::table)
        .values(&NewConversationMember {
            conversation_id: created.id,
            user_id: creator_user_id,
            joined_at: now,
        })
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(created)
}

/// Get all conversation member user IDs for a conversation.
pub async fn member_user_ids(conversation_id: Uuid) -> Result<Vec<i32>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let ids = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<i32>(&mut conn)
        .await?;
    Ok(ids)
}

/// Check if a user is a member of a conversation.
pub async fn is_member(
    conversation_id: Uuid,
    user_id: i32,
) -> Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let count = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(user_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await?;
    Ok(count > 0)
}

/// Load conversations for a user with latest message info for the room list.
pub async fn list_for_user(
    user_id: i32,
) -> Result<Vec<super::protocol::ConversationPayload>, crate::error::AppError> {
    use super::protocol::ConversationPayload;
    use crate::db::schema::messages;

    let mut conn = crate::db::conn().await?;

    // Get all conversation IDs the user is a member of
    let conv_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_id))
        .select(conversation_members::conversation_id)
        .load(&mut conn)
        .await?;

    if conv_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Load conversations
    let convs: Vec<Conversation> = conversations::table
        .filter(conversations::id.eq_any(&conv_ids))
        .load(&mut conn)
        .await?;

    // Load read watermarks
    let read_marks: Vec<(Uuid, Option<Uuid>)> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_id))
        .filter(conversation_members::conversation_id.eq_any(&conv_ids))
        .select((
            conversation_members::conversation_id,
            conversation_members::last_read_message_id,
        ))
        .load(&mut conn)
        .await?;

    let read_map: std::collections::HashMap<Uuid, Option<Uuid>> = read_marks.into_iter().collect();

    let mut payloads = Vec::with_capacity(convs.len());

    for conv in &convs {
        // Latest non-deleted message
        let latest: Option<crate::db::models::messages::Message> = messages::table
            .filter(messages::conversation_id.eq(conv.id))
            .filter(messages::deleted_at.is_null())
            .order(messages::created_at.desc())
            .first(&mut conn)
            .await
            .optional()?;

        // Unread count
        let unread_count = if let Some(Some(last_read_id)) = read_map.get(&conv.id) {
            // Count messages after the last read message
            let last_read_ts: Option<chrono::DateTime<chrono::Utc>> = messages::table
                .filter(messages::id.eq(last_read_id))
                .select(messages::created_at)
                .first(&mut conn)
                .await
                .optional()?;

            if let Some(ts) = last_read_ts {
                messages::table
                    .filter(messages::conversation_id.eq(conv.id))
                    .filter(messages::deleted_at.is_null())
                    .filter(messages::created_at.gt(ts))
                    .filter(messages::sender_id.ne(user_id))
                    .count()
                    .get_result::<i64>(&mut conn)
                    .await?
            } else {
                0
            }
        } else {
            // No read watermark = all messages from others are unread
            messages::table
                .filter(messages::conversation_id.eq(conv.id))
                .filter(messages::deleted_at.is_null())
                .filter(messages::sender_id.ne(user_id))
                .count()
                .get_result::<i64>(&mut conn)
                .await?
        };

        // For DMs, resolve the other user's profile
        let (direct_user_id, direct_user_name, direct_user_avatar) = if conv.kind == "dm" {
            let other_user_id = if conv.user_low_id == Some(user_id) {
                conv.user_high_id
            } else {
                conv.user_low_id
            };
            if let Some(other_id) = other_user_id {
                let profile: Option<(i32, String, Option<String>)> = profiles::table
                    .inner_join(users::table.on(users::id.eq(profiles::user_id)))
                    .filter(users::id.eq(other_id))
                    .select((users::id, profiles::name, profiles::profile_picture))
                    .first(&mut conn)
                    .await
                    .optional()?;
                match profile {
                    Some((uid, name, avatar)) => {
                        let avatar_url = avatar.as_ref().map(|filename| {
                            crate::api::imgproxy_signing::signed_url(filename, "thumb", "webp")
                                .unwrap_or_else(|| format!("/api/v1/uploads/{filename}"))
                        });
                        (Some(uid.to_string()), Some(name), avatar_url)
                    }
                    None => (None, None, None),
                }
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        // Resolve latest message sender name
        let latest_sender_name = if let Some(ref msg) = latest {
            let name: Option<String> = profiles::table
                .inner_join(users::table.on(users::id.eq(profiles::user_id)))
                .filter(users::id.eq(msg.sender_id))
                .select(profiles::name)
                .first(&mut conn)
                .await
                .optional()?;
            name
        } else {
            None
        };

        payloads.push(ConversationPayload {
            id: conv.id,
            kind: conv.kind.clone(),
            title: conv.title.clone(),
            is_direct: conv.kind == "dm",
            direct_user_id,
            direct_user_name,
            direct_user_avatar,
            unread_count,
            latest_message: latest.as_ref().map(|m| m.body.clone()),
            latest_timestamp: latest.as_ref().map(|m| m.created_at.to_rfc3339()),
            latest_message_is_mine: latest.as_ref().is_some_and(|m| m.sender_id == user_id),
            latest_sender_name,
        });
    }

    // Sort by latest message timestamp descending
    payloads.sort_by(|a, b| b.latest_timestamp.cmp(&a.latest_timestamp));

    Ok(payloads)
}

/// Sync event membership: add or remove a user from the event conversation.
/// Called from outbox job dispatch.
#[allow(clippy::similar_names)]
pub async fn sync_event_membership(
    event_id: Uuid,
    profile_id: Uuid,
    leave: bool,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn()
        .await
        .map_err(|e| format!("db conn: {e}"))?;

    // Resolve user_id from profile_id
    let user_id: Option<i32> = profiles::table
        .filter(profiles::id.eq(profile_id))
        .select(profiles::user_id)
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| format!("resolve user_id: {e}"))?;

    let Some(user_id) = user_id else {
        return Err(format!("profile {profile_id} not found"));
    };

    if leave {
        // Remove from conversation if it exists
        let conv: Option<Conversation> = conversations::table
            .filter(conversations::kind.eq("event"))
            .filter(conversations::event_id.eq(event_id))
            .first(&mut conn)
            .await
            .optional()
            .map_err(|e| format!("find conversation: {e}"))?;

        if let Some(conv) = conv {
            diesel::delete(
                conversation_members::table
                    .filter(conversation_members::conversation_id.eq(conv.id))
                    .filter(conversation_members::user_id.eq(user_id)),
            )
            .execute(&mut conn)
            .await
            .map_err(|e| format!("delete member: {e}"))?;
        }
    } else {
        // Resolve event title
        let event_row: Option<(String, Uuid)> = events::table
            .filter(events::id.eq(event_id))
            .select((events::title, events::creator_id))
            .first(&mut conn)
            .await
            .optional()
            .map_err(|e| format!("load event: {e}"))?;

        let Some((event_title, creator_profile_id)) = event_row else {
            return Err(format!("event {event_id} not found"));
        };

        // Resolve creator user_id
        let creator_user_id: i32 = profiles::table
            .filter(profiles::id.eq(creator_profile_id))
            .select(profiles::user_id)
            .first(&mut conn)
            .await
            .map_err(|e| format!("resolve creator user_id: {e}"))?;

        // Ensure conversation exists
        let conv = resolve_or_create_event_conversation(event_id, &event_title, creator_user_id)
            .await
            .map_err(|e| format!("resolve conversation: {e}"))?;

        // Add member
        diesel::insert_into(conversation_members::table)
            .values(&NewConversationMember {
                conversation_id: conv.id,
                user_id,
                joined_at: Utc::now(),
            })
            .on_conflict_do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| format!("insert member: {e}"))?;
    }

    Ok(())
}
