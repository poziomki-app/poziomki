use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Uuid as SqlUuid};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db;
use crate::db::models::conversation_members::NewConversationMember;
use crate::db::models::conversations::{Conversation, NewConversation};
use crate::db::schema::{conversation_members, conversations, events, profile_blocks, profiles};

/// Fetch the canonical DM conversation for `(low, high)` via the
/// SECURITY DEFINER helper installed in the Tier-B migration. Bypasses
/// `conversations_viewer` RLS so callers can discover a concurrently
/// created row whose membership hasn't yet been inserted — the resolve
/// flow would otherwise dead-end on `NotFound` after its race fallback.
async fn find_dm_conversation(
    conn: &mut AsyncPgConnection,
    low: i32,
    high: i32,
) -> Result<Option<Conversation>, diesel::result::Error> {
    diesel::sql_query("SELECT * FROM app.find_dm_conversation($1, $2)")
        .bind::<Integer, _>(low)
        .bind::<Integer, _>(high)
        .get_result::<Conversation>(conn)
        .await
        .optional()
}

/// Event-chat counterpart to `find_dm_conversation`.
async fn find_event_conversation(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
) -> Result<Option<Conversation>, diesel::result::Error> {
    diesel::sql_query("SELECT * FROM app.find_event_conversation($1)")
        .bind::<SqlUuid, _>(event_id)
        .get_result::<Conversation>(conn)
        .await
        .optional()
}

/// Resolve or create a DM conversation between two users.
/// Uses canonical pair (lower id, higher id) for uniqueness.
#[allow(clippy::similar_names)]
pub async fn resolve_or_create_dm(
    conn: &mut AsyncPgConnection,
    first_user_id: i32,
    second_user_id: i32,
) -> Result<Conversation, crate::error::AppError> {
    let (low, high) = if first_user_id < second_user_id {
        (first_user_id, second_user_id)
    } else {
        (second_user_id, first_user_id)
    };

    // Try to find existing DM. The SD lookup bypasses
    // `conversations_viewer` so viewers who aren't yet members (e.g.
    // the resolve flow before membership is bootstrapped) still see
    // the row.
    if let Some(found) = find_dm_conversation(conn, low, high).await? {
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
        .get_result::<Conversation>(conn)
        .await
        .optional()?;

    // Handle race condition: another request may have created it. The
    // fallback uses the SD helper so RLS doesn't filter the row out
    // when the viewer's membership hasn't been inserted yet.
    let created = match inserted {
        Some(c) => c,
        None => find_dm_conversation(conn, low, high)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::message(format!(
                    "DM conversation race-fallback lookup returned None for pair ({low}, {high})"
                ))
            })?,
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
        .execute(conn)
        .await?;

    Ok(created)
}

/// Resolve or create an event conversation.
pub async fn resolve_or_create_event_conversation(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    event_title: &str,
    creator_user_id: i32,
) -> Result<Conversation, crate::error::AppError> {
    // Existing-row lookup via SD helper so an attendee resolving the
    // chat before their membership has been inserted still sees the
    // row created by the event owner (or an earlier attendee).
    if let Some(found) = find_event_conversation(conn, event_id).await? {
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
        .get_result::<Conversation>(conn)
        .await
        .optional()?;

    let created = match inserted {
        Some(c) => c,
        None => find_event_conversation(conn, event_id)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::message(format!(
                    "event conversation race-fallback lookup returned None for event {event_id}"
                ))
            })?,
    };

    // Add creator as first member
    diesel::insert_into(conversation_members::table)
        .values(&NewConversationMember {
            conversation_id: created.id,
            user_id: creator_user_id,
            joined_at: now,
        })
        .on_conflict_do_nothing()
        .execute(conn)
        .await?;

    Ok(created)
}

/// Get all conversation member user IDs for a conversation.
pub async fn member_user_ids(
    conn: &mut AsyncPgConnection,
    conversation_id: Uuid,
) -> Result<Vec<i32>, crate::error::AppError> {
    let ids = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<i32>(conn)
        .await?;
    Ok(ids)
}

/// Check if a user is a member of a conversation.
pub async fn is_member(
    conn: &mut AsyncPgConnection,
    conversation_id: Uuid,
    user_id: i32,
) -> Result<bool, crate::error::AppError> {
    let count = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(user_id))
        .count()
        .get_result::<i64>(conn)
        .await?;
    Ok(count > 0)
}

/// Load conversations for a user with latest message info for the room list.
///
/// Uses batch queries instead of per-conversation loops to avoid N+1.
#[allow(clippy::similar_names, clippy::too_many_lines)]
pub async fn list_for_user(
    conn: &mut AsyncPgConnection,
    user_id: i32,
    viewer_is_stub: bool,
) -> Result<Vec<super::protocol::ConversationPayload>, crate::error::AppError> {
    use super::protocol::ConversationPayload;
    use std::collections::HashMap;
    // (user_id, profile_id, name, avatar, status_text, status_emoji, status_expires_at)
    type ProfileRow = (
        i32,
        uuid::Uuid,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    );
    type ProfileMap = HashMap<
        i32,
        (
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >;

    let conv_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_id))
        .select(conversation_members::conversation_id)
        .load(conn)
        .await?;

    if conv_ids.is_empty() {
        return Ok(Vec::new());
    }

    let all_convs: Vec<Conversation> = conversations::table
        .filter(conversations::id.eq_any(&conv_ids))
        .load(conn)
        .await?;

    // Collect the "other participant" id for every DM so we can batch-load
    // their is_review_stub flags in a single query.
    let other_ids: Vec<i32> = all_convs
        .iter()
        .filter(|c| c.kind == "dm")
        .filter_map(|c| {
            if c.user_low_id == Some(user_id) {
                c.user_high_id
            } else {
                c.user_low_id
            }
        })
        .collect();

    // Narrow public projection: is_review_stub only, no full users row.
    let stub_flags: std::collections::HashMap<i32, bool> = db::user_review_stubs(conn, &other_ids)
        .await?
        .into_iter()
        .map(|r| (r.user_id, r.is_review_stub))
        .collect();

    // Hide DMs where the other participant has a different is_review_stub
    // value so stub accounts stay invisible to real users (and vice versa).
    let convs: Vec<Conversation> = all_convs
        .into_iter()
        .filter(|conv| {
            if conv.kind != "dm" {
                return true;
            }
            let other_id = if conv.user_low_id == Some(user_id) {
                conv.user_high_id
            } else {
                conv.user_low_id
            };
            other_id
                .is_none_or(|id| stub_flags.get(&id).copied().unwrap_or(false) == viewer_is_stub)
        })
        .collect();

    let conv_ids: Vec<Uuid> = convs.iter().map(|c| c.id).collect();
    if conv_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Batch-load latest message per conversation (DISTINCT ON)
    let latest_messages: Vec<crate::db::models::messages::Message> = diesel::sql_query(
        "SELECT DISTINCT ON (conversation_id) \
                 id, conversation_id, sender_id, body, kind, \
                 attachment_upload_id, reply_to_id, client_id, \
                 edited_at, deleted_at, created_at \
             FROM messages \
             WHERE conversation_id = ANY($1) AND deleted_at IS NULL \
             ORDER BY conversation_id, created_at DESC, id DESC",
    )
    .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&conv_ids)
    .load(conn)
    .await?;

    let latest_map: HashMap<Uuid, &crate::db::models::messages::Message> = latest_messages
        .iter()
        .map(|m| (m.conversation_id, m))
        .collect();

    // Unread counts
    //    For conversations with a read watermark: count messages after that timestamp
    //    For conversations without: count all messages from others
    let unread_counts: Vec<UnreadCountRow> = diesel::sql_query(
        "SELECT m.conversation_id, COUNT(*) as cnt \
             FROM messages m \
             INNER JOIN conversation_members cm \
                 ON cm.conversation_id = m.conversation_id AND cm.user_id = $1 \
             LEFT JOIN messages rm ON rm.id = cm.last_read_message_id \
             WHERE m.conversation_id = ANY($2) \
               AND m.deleted_at IS NULL \
               AND m.sender_id != $1 \
               AND (rm.id IS NULL OR m.created_at > rm.created_at \
                    OR (m.created_at = rm.created_at AND m.id > rm.id)) \
               AND (rm.id IS NULL OR m.id != cm.last_read_message_id) \
             GROUP BY m.conversation_id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&conv_ids)
    .load::<UnreadCountRow>(conn)
    .await?;

    let unread_map: HashMap<Uuid, i64> = unread_counts
        .into_iter()
        .map(|r| (r.conversation_id, r.cnt))
        .collect();

    let mut profile_user_ids: Vec<i32> = Vec::new();
    for conv in &convs {
        if conv.kind == "dm" {
            let other = if conv.user_low_id == Some(user_id) {
                conv.user_high_id
            } else {
                conv.user_low_id
            };
            if let Some(id) = other {
                profile_user_ids.push(id);
            }
        }
    }
    for msg in &latest_messages {
        profile_user_ids.push(msg.sender_id);
    }
    profile_user_ids.sort_unstable();
    profile_user_ids.dedup();

    // Batch-load profiles (filter on profiles.user_id directly — no need
    // to join users, which would require broad SELECT on a sensitive table).
    let profile_rows: Vec<ProfileRow> = if profile_user_ids.is_empty() {
        Vec::new()
    } else {
        profiles::table
            .filter(profiles::user_id.eq_any(&profile_user_ids))
            .select((
                profiles::user_id,
                profiles::id,
                profiles::name,
                profiles::profile_picture,
                profiles::status_text,
                profiles::status_emoji,
                profiles::status_expires_at,
            ))
            .load(conn)
            .await?
    };

    let profile_map: ProfileMap = profile_rows
        .into_iter()
        .map(|(uid, pid, name, avatar, status, emoji, expires)| {
            (uid, (pid, name, avatar, status, emoji, expires))
        })
        .collect();

    // Resolve current user's profile ID for block checks
    let my_profile_id: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .select(profiles::id)
        .first(conn)
        .await
        .optional()?;

    // Batch-load blocked profile IDs (profiles I blocked or that blocked me)
    let blocked_profile_ids: std::collections::HashSet<uuid::Uuid> =
        if let Some(my_pid) = my_profile_id {
            let rows: Vec<(uuid::Uuid, uuid::Uuid)> = profile_blocks::table
                .filter(
                    profile_blocks::blocker_id
                        .eq(my_pid)
                        .or(profile_blocks::blocked_id.eq(my_pid)),
                )
                .select((profile_blocks::blocker_id, profile_blocks::blocked_id))
                .load(conn)
                .await?;
            rows.into_iter()
                .flat_map(|(a, b)| {
                    // Return the "other" profile ID (not mine)
                    if a == my_pid {
                        vec![b]
                    } else {
                        vec![a]
                    }
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };

    let mut payloads = Vec::with_capacity(convs.len());
    for conv in &convs {
        let latest = latest_map.get(&conv.id).copied();
        let unread_count = unread_map.get(&conv.id).copied().unwrap_or(0);

        // DM profile. Status is filtered by expiry here (read-time TTL).
        let now = chrono::Utc::now();
        let (
            direct_user_id,
            direct_user_pid,
            direct_user_name,
            direct_user_avatar,
            direct_user_status,
            direct_user_status_emoji,
        ) = if conv.kind == "dm" {
            let other_id = if conv.user_low_id == Some(user_id) {
                conv.user_high_id
            } else {
                conv.user_low_id
            };
            other_id
                .and_then(|oid| {
                    profile_map
                        .get(&oid)
                        .map(|(pid, name, avatar, status, emoji, expires)| {
                            let avatar_url = avatar.as_ref().map(|filename| {
                                crate::api::imgproxy_signing::signed_avatar_url(filename)
                                    .unwrap_or_else(|| format!("/api/v1/uploads/{filename}"))
                            });
                            let live = expires.is_none_or(|exp| exp > now);
                            let (status_out, emoji_out) = if live {
                                (status.clone(), emoji.clone())
                            } else {
                                (None, None)
                            };
                            (
                                Some(oid.to_string()),
                                Some(pid.to_string()),
                                Some(name.clone()),
                                avatar_url,
                                status_out,
                                emoji_out,
                            )
                        })
                })
                .unwrap_or((None, None, None, None, None, None))
        } else {
            (None, None, None, None, None, None)
        };

        // Latest message sender name
        let latest_sender_name = latest.and_then(|msg| {
            profile_map
                .get(&msg.sender_id)
                .map(|(_, name, _, _, _, _)| name.clone())
        });

        // Check if the DM partner is blocked (either direction)
        let is_blocked = if conv.kind == "dm" {
            let other_id = if conv.user_low_id == Some(user_id) {
                conv.user_high_id
            } else {
                conv.user_low_id
            };
            other_id
                .and_then(|oid| profile_map.get(&oid))
                .is_some_and(|(pid, _, _, _, _, _)| blocked_profile_ids.contains(pid))
        } else {
            false
        };

        payloads.push(ConversationPayload {
            id: conv.id,
            kind: conv.kind.clone(),
            title: conv.title.clone(),
            is_direct: conv.kind == "dm",
            direct_user_id,
            direct_user_pid,
            direct_user_name,
            direct_user_avatar,
            direct_user_status,
            direct_user_status_emoji,
            unread_count,
            latest_message: latest.map(|m| m.body.clone()),
            latest_timestamp: latest.map(|m| m.created_at.to_rfc3339()),
            latest_message_is_mine: latest.is_some_and(|m| m.sender_id == user_id),
            latest_sender_name,
            is_blocked,
        });
    }

    // Sort by latest message timestamp descending
    payloads.sort_by(|a, b| b.latest_timestamp.cmp(&a.latest_timestamp));

    Ok(payloads)
}

/// Helper struct for raw SQL unread count query.
#[derive(diesel::QueryableByName)]
struct UnreadCountRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    conversation_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    cnt: i64,
}

/// Sync event membership: add or remove a user from the event conversation.
/// Called from outbox job dispatch (worker, BYPASSRLS), so it opens its own
/// connection rather than requiring a viewer context.
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
        let conv = resolve_or_create_event_conversation(
            &mut conn,
            event_id,
            &event_title,
            creator_user_id,
        )
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
