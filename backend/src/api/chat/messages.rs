use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::protocol::{MessagePayload, ReactionPayload, ReplyPayload};
use crate::db::models::message_reactions::NewMessageReaction;
use crate::db::models::messages::{Message, NewMessage};
use crate::db::schema::{message_reactions, messages, profiles, users};

/// Convert a single message to a payload via the batch path.
async fn single_message_payload(
    msg: &Message,
    viewer_user_id: i32,
    conn: &mut crate::db::DbConn,
) -> Result<MessagePayload, crate::error::AppError> {
    batch_messages_to_payloads(std::slice::from_ref(msg), viewer_user_id, conn)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::AppError::message("payload conversion failed"))
}

/// Insert a new message and return a serializable payload.
/// If `client_id` is set and a message already exists with the same
/// `(conversation_id, client_id)`, return the existing message (idempotent).
/// Returns `(message, payload, created)` where `created` is `false` on dedup hit.
pub async fn create_message(
    conversation_id: Uuid,
    sender_id: i32,
    body: &str,
    kind: &str,
    reply_to_id: Option<Uuid>,
    client_id: Option<String>,
) -> Result<(Message, MessagePayload, bool), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    // Dedup: if client_id is set, check for an existing message first
    if let Some(ref cid) = client_id {
        let existing: Option<Message> = messages::table
            .filter(messages::conversation_id.eq(conversation_id))
            .filter(messages::client_id.eq(cid))
            .filter(messages::deleted_at.is_null())
            .first(&mut conn)
            .await
            .optional()?;
        if let Some(msg) = existing {
            let payload = single_message_payload(&msg, 0, &mut conn).await?;
            return Ok((msg, payload, false));
        }
    }

    let now = Utc::now();
    let msg_id = Uuid::new_v4();

    // Validate reply_to_id belongs to same conversation
    if let Some(reply_id) = reply_to_id {
        let reply_in_conv = messages::table
            .filter(messages::id.eq(reply_id))
            .filter(messages::conversation_id.eq(conversation_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await?
            > 0;
        if !reply_in_conv {
            return Err(crate::error::AppError::message(
                "reply_to message does not belong to this conversation",
            ));
        }
    }

    let new = NewMessage {
        id: msg_id,
        conversation_id,
        sender_id,
        body: body.to_string(),
        kind: kind.to_string(),
        reply_to_id,
        client_id: client_id.clone(),
        created_at: now,
    };

    let msg = match diesel::insert_into(messages::table)
        .values(&new)
        .get_result::<Message>(&mut conn)
        .await
    {
        Ok(msg) => msg,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) if client_id.is_some() => {
            // Race: another insert won, fetch the existing message
            // SAFETY: guard ensures client_id is Some
            let cid = client_id
                .as_ref()
                .ok_or_else(|| crate::error::AppError::message("client_id disappeared"))?;
            let msg = messages::table
                .filter(messages::conversation_id.eq(conversation_id))
                .filter(messages::client_id.eq(cid))
                .filter(messages::deleted_at.is_null())
                .first(&mut conn)
                .await?;
            let payload = single_message_payload(&msg, 0, &mut conn).await?;
            return Ok((msg, payload, false));
        }
        Err(e) => return Err(e.into()),
    };

    // viewer_user_id=0 so broadcast payload has is_mine=false for all recipients.
    // Each client computes isMine locally; sender matches via client_id.
    let payload = single_message_payload(&msg, 0, &mut conn).await?;
    Ok((msg, payload, true))
}

/// Edit a message body. Returns the updated message.
pub async fn edit_message(
    message_id: Uuid,
    sender_id: i32,
    new_body: &str,
) -> Result<Message, crate::error::AppError> {
    if new_body.trim().is_empty() {
        return Err(crate::error::AppError::message(
            "message body cannot be empty",
        ));
    }
    if new_body.len() > 10_000 {
        return Err(crate::error::AppError::message("message body too long"));
    }

    let mut conn = crate::db::conn().await?;
    let now = Utc::now();

    let updated = diesel::update(
        messages::table
            .filter(messages::id.eq(message_id))
            .filter(messages::sender_id.eq(sender_id))
            .filter(messages::deleted_at.is_null()),
    )
    .set((messages::body.eq(new_body), messages::edited_at.eq(now)))
    .get_result::<Message>(&mut conn)
    .await
    .optional()?;

    updated.ok_or_else(|| crate::error::AppError::message("message not found or not editable"))
}

/// Soft-delete a message.
pub async fn delete_message(
    message_id: Uuid,
    sender_id: i32,
) -> Result<Message, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let now = Utc::now();

    let updated = diesel::update(
        messages::table
            .filter(messages::id.eq(message_id))
            .filter(messages::sender_id.eq(sender_id))
            .filter(messages::deleted_at.is_null()),
    )
    .set(messages::deleted_at.eq(Some(now)))
    .get_result::<Message>(&mut conn)
    .await
    .optional()?;

    updated.ok_or_else(|| crate::error::AppError::message("message not found or already deleted"))
}

/// Toggle a reaction. Returns (added: bool, message) — true if added, false if removed.
pub async fn toggle_reaction(
    message_id: Uuid,
    user_id: i32,
    emoji: &str,
) -> Result<(bool, Message), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    // Check the message exists
    let msg = messages::table
        .filter(messages::id.eq(message_id))
        .filter(messages::deleted_at.is_null())
        .first::<Message>(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate::error::AppError::message("message not found"))?;

    // Try to delete existing reaction
    let deleted = diesel::delete(
        message_reactions::table
            .filter(message_reactions::message_id.eq(message_id))
            .filter(message_reactions::user_id.eq(user_id))
            .filter(message_reactions::emoji.eq(emoji)),
    )
    .execute(&mut conn)
    .await?;

    if deleted > 0 {
        return Ok((false, msg));
    }

    // Insert new reaction
    let now = Utc::now();
    diesel::insert_into(message_reactions::table)
        .values(&NewMessageReaction {
            id: Uuid::new_v4(),
            message_id,
            user_id,
            emoji: emoji.to_string(),
            created_at: now,
        })
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    Ok((true, msg))
}

/// Update read watermark for a user in a conversation.
pub async fn mark_read(
    conversation_id: Uuid,
    user_id: i32,
    message_id: Uuid,
) -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    // Verify message belongs to this conversation
    let belongs = messages::table
        .filter(messages::id.eq(message_id))
        .filter(messages::conversation_id.eq(conversation_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await?
        > 0;

    if !belongs {
        return Err(crate::error::AppError::message(
            "message does not belong to this conversation",
        ));
    }

    // Only advance watermark forward (never move backwards).
    // Uses (created_at, id) compound comparison to match unread count query.
    diesel::sql_query(
        "UPDATE conversation_members SET last_read_message_id = $1 \
         WHERE conversation_id = $2 AND user_id = $3 \
           AND (last_read_message_id IS NULL \
                OR (SELECT created_at FROM messages WHERE id = $1) \
                 > (SELECT created_at FROM messages WHERE id = last_read_message_id) \
                OR ((SELECT created_at FROM messages WHERE id = $1) \
                  = (SELECT created_at FROM messages WHERE id = last_read_message_id) \
                 AND $1 > last_read_message_id))",
    )
    .bind::<diesel::sql_types::Uuid, _>(message_id)
    .bind::<diesel::sql_types::Uuid, _>(conversation_id)
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .execute(&mut conn)
    .await?;

    Ok(())
}

/// Look up the `conversation_id` for a given message.
pub async fn message_conversation_id(
    message_id: Uuid,
) -> Result<Option<Uuid>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let cid = messages::table
        .filter(messages::id.eq(message_id))
        .select(messages::conversation_id)
        .first::<Uuid>(&mut conn)
        .await
        .optional()?;
    Ok(cid)
}

/// Load message history for a conversation, paginated backwards.
pub async fn load_history(
    conversation_id: Uuid,
    before: Option<Uuid>,
    limit: i64,
    viewer_user_id: i32,
) -> Result<(Vec<MessagePayload>, bool), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let query_limit = limit + 1; // fetch one extra to determine has_more

    let msgs: Vec<Message> = if let Some(before_id) = before {
        let before_ts: Option<chrono::DateTime<chrono::Utc>> = messages::table
            .filter(messages::id.eq(before_id))
            .filter(messages::conversation_id.eq(conversation_id))
            .select(messages::created_at)
            .first(&mut conn)
            .await
            .optional()?;
        let Some(before_ts) = before_ts else {
            return Ok((Vec::new(), false));
        };

        messages::table
            .filter(messages::conversation_id.eq(conversation_id))
            .filter(messages::deleted_at.is_null())
            .filter(
                messages::created_at.lt(before_ts).or(messages::created_at
                    .eq(before_ts)
                    .and(messages::id.lt(before_id))),
            )
            .order((messages::created_at.desc(), messages::id.desc()))
            .limit(query_limit)
            .load(&mut conn)
            .await?
    } else {
        messages::table
            .filter(messages::conversation_id.eq(conversation_id))
            .filter(messages::deleted_at.is_null())
            .order((messages::created_at.desc(), messages::id.desc()))
            .limit(query_limit)
            .load(&mut conn)
            .await?
    };

    let limit_usize = usize::try_from(limit).unwrap_or(0);
    let has_more = msgs.len() > limit_usize;
    let msgs: Vec<Message> = msgs.into_iter().take(limit_usize).collect();

    let mut payloads = batch_messages_to_payloads(&msgs, viewer_user_id, &mut conn).await?;

    // Return in chronological order (oldest first)
    payloads.reverse();

    Ok((payloads, has_more))
}

/// Batch-convert messages to payloads, resolving all related data upfront
/// instead of N+1 per-message queries.
async fn batch_messages_to_payloads(
    msgs: &[Message],
    viewer_user_id: i32,
    conn: &mut crate::db::DbConn,
) -> Result<Vec<MessagePayload>, crate::error::AppError> {
    use std::collections::HashMap;

    if msgs.is_empty() {
        return Ok(Vec::new());
    }

    // Collect all IDs we need to resolve
    let sender_ids: Vec<i32> = msgs.iter().map(|m| m.sender_id).collect();
    let msg_ids: Vec<Uuid> = msgs.iter().map(|m| m.id).collect();
    let reply_ids: Vec<Uuid> = msgs.iter().filter_map(|m| m.reply_to_id).collect();

    // Batch-load sender profiles
    let sender_rows: Vec<(i32, String, Uuid, Option<String>)> = profiles::table
        .inner_join(users::table.on(users::id.eq(profiles::user_id)))
        .filter(users::id.eq_any(&sender_ids))
        .select((
            users::id,
            profiles::name,
            users::pid,
            profiles::profile_picture,
        ))
        .load(conn)
        .await?;
    let sender_map: HashMap<i32, (String, Uuid, Option<String>)> = sender_rows
        .into_iter()
        .map(|(id, name, pid, avatar)| (id, (name, pid, avatar)))
        .collect();

    // Batch-load reply messages
    let reply_msgs: HashMap<Uuid, Message> = if reply_ids.is_empty() {
        HashMap::new()
    } else {
        messages::table
            .filter(messages::id.eq_any(&reply_ids))
            .load::<Message>(conn)
            .await?
            .into_iter()
            .map(|m| (m.id, m))
            .collect()
    };

    // Collect reply sender IDs for name resolution
    let reply_sender_ids: Vec<i32> = reply_msgs.values().map(|m| m.sender_id).collect();
    let reply_sender_names: HashMap<i32, String> = if reply_sender_ids.is_empty() {
        HashMap::new()
    } else {
        profiles::table
            .inner_join(users::table.on(users::id.eq(profiles::user_id)))
            .filter(users::id.eq_any(&reply_sender_ids))
            .select((users::id, profiles::name))
            .load::<(i32, String)>(conn)
            .await?
            .into_iter()
            .collect()
    };

    // Batch-load reactions
    let raw_reactions: Vec<(Uuid, String, i32)> = message_reactions::table
        .filter(message_reactions::message_id.eq_any(&msg_ids))
        .select((
            message_reactions::message_id,
            message_reactions::emoji,
            message_reactions::user_id,
        ))
        .load(conn)
        .await?;

    // Batch-load reaction user names
    let reaction_user_ids: Vec<i32> = raw_reactions.iter().map(|(_, _, uid)| *uid).collect();
    let reaction_user_names: HashMap<i32, String> = if reaction_user_ids.is_empty() {
        HashMap::new()
    } else {
        profiles::table
            .inner_join(users::table.on(users::id.eq(profiles::user_id)))
            .filter(users::id.eq_any(&reaction_user_ids))
            .select((users::id, profiles::name))
            .load::<(i32, String)>(conn)
            .await?
            .into_iter()
            .collect()
    };

    // Group reactions by message_id
    let mut reactions_by_msg: HashMap<Uuid, Vec<(String, i32)>> = HashMap::new();
    for (mid, emoji, uid) in raw_reactions {
        reactions_by_msg.entry(mid).or_default().push((emoji, uid));
    }

    // Assemble payloads
    let mut payloads = Vec::with_capacity(msgs.len());
    for msg in msgs {
        let (sender_name, sender_pid, sender_avatar) = sender_map.get(&msg.sender_id).map_or_else(
            || ("Unknown".to_string(), None, None),
            |(name, pid, avatar)| {
                let avatar_url = avatar
                    .as_ref()
                    .and_then(|f| crate::api::imgproxy_signing::signed_avatar_url(f));
                (name.clone(), Some(pid.to_string()), avatar_url)
            },
        );

        let reply_to = msg.reply_to_id.and_then(|rid| {
            reply_msgs.get(&rid).and_then(|rm| {
                // Only resolve replies within the same conversation
                if rm.conversation_id != msg.conversation_id {
                    return None;
                }
                Some(ReplyPayload {
                    message_id: rm.id,
                    sender_name: reply_sender_names.get(&rm.sender_id).cloned(),
                    body: if rm.deleted_at.is_some() {
                        None
                    } else {
                        Some(rm.body.clone())
                    },
                })
            })
        });

        // Build reactions for this message
        let reactions = reactions_by_msg
            .get(&msg.id)
            .map_or_else(Vec::new, |msg_reactions| {
                let mut reaction_map: HashMap<String, (Vec<i32>, Vec<String>, bool)> =
                    HashMap::new();
                for (emoji, uid) in msg_reactions {
                    let entry = reaction_map.entry(emoji.clone()).or_default();
                    entry.0.push(*uid);
                    entry.1.push(
                        reaction_user_names
                            .get(uid)
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string()),
                    );
                    if *uid == viewer_user_id {
                        entry.2 = true;
                    }
                }
                reaction_map
                    .into_iter()
                    .map(
                        |(emoji, (user_ids, sender_names, reacted_by_me))| ReactionPayload {
                            count: i64::try_from(user_ids.len()).unwrap_or(0),
                            emoji,
                            reacted_by_me,
                            user_ids,
                            sender_names,
                        },
                    )
                    .collect()
            });

        payloads.push(MessagePayload {
            id: msg.id,
            conversation_id: msg.conversation_id,
            sender_id: msg.sender_id,
            sender_pid,
            sender_name,
            sender_avatar,
            body: msg.body.clone(),
            kind: msg.kind.clone(),
            reply_to,
            reactions,
            client_id: msg.client_id.clone(),
            is_mine: msg.sender_id == viewer_user_id,
            is_edited: msg.edited_at.is_some(),
            created_at: msg.created_at.to_rfc3339(),
        });
    }

    Ok(payloads)
}
