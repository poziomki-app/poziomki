use axum::extract::ws::{Message, WebSocket};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use super::conversations;
use super::hub::ChatHub;
use super::messages as chat_messages;
use super::protocol::{ClientMessage, ServerMessage};
use crate::api::state::hash_session_token;
use crate::db::schema::{conversations as conversations_table, profile_blocks, profiles};

const AUTH_TIMEOUT_SECS: u64 = 5;
const DEFAULT_HISTORY_LIMIT: i64 = 50;
const MAX_HISTORY_LIMIT: i64 = 200;
const SEND_RATE_LIMIT: u32 = 10;
/// Interval at which the server pings idle sockets so half-open TCP
/// connections are reaped without waiting on kernel keepalive.
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
/// Close the socket if no frame (including pong) arrives within this window.
const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// Handle a WebSocket connection after Axum upgrade.
pub async fn handle_socket(socket: WebSocket, hub: ChatHub) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // --- Step 1: Authenticate ---
    let Some((user_id, _user_pid)) = authenticate(&mut ws_tx, &mut ws_rx).await else {
        return;
    };

    // --- Step 2: Register in hub ---
    let mut hub_rx = hub.register(user_id);
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Send initial conversation list
    let conv_list = conversations::list_for_user(user_id)
        .await
        .unwrap_or_default();
    let init_msg = ServerMessage::Conversations {
        conversations: conv_list,
    };
    if send_json(&mut ws_tx, &init_msg).await.is_err() {
        drop(hub_rx);
        hub.unregister(user_id);
        return;
    }

    // --- Step 3: Main loop ---
    // Spawn a writer task that drains hub messages + outbound messages
    let writer_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = hub_rx.recv() => {
                    if send_json(&mut ws_tx, &msg).await.is_err() {
                        break;
                    }
                }
                Some(msg) = outbound_rx.recv() => {
                    if send_json(&mut ws_tx, &msg).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    // Reader loop: process incoming client messages + server heartbeat.
    let mut send_count: u32 = 0;
    let mut send_window = tokio::time::Instant::now();
    let mut last_seen = tokio::time::Instant::now();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    // First tick fires immediately — skip it so we don't ping a freshly-
    // opened socket before the client has settled.
    heartbeat.tick().await;

    loop {
        tokio::select! {
            frame = ws_rx.next() => {
                let Some(Ok(frame)) = frame else { break };
                last_seen = tokio::time::Instant::now();
                match frame {
                    Message::Text(text) => {
                        let parsed: Result<ClientMessage, _> = serde_json::from_str(&text);
                        match parsed {
                            Ok(client_msg) => {
                                // Rate-limit Send messages (10/sec)
                                if matches!(&client_msg, ClientMessage::Send { .. }) {
                                    if send_window.elapsed() >= std::time::Duration::from_secs(1) {
                                        send_count = 0;
                                        send_window = tokio::time::Instant::now();
                                    }
                                    send_count += 1;
                                    if send_count > SEND_RATE_LIMIT {
                                        let _ = outbound_tx.send(ServerMessage::Error {
                                            message: "rate limited: too many messages".to_string(),
                                        });
                                        continue;
                                    }
                                }
                                handle_client_message(client_msg, user_id, &hub, &outbound_tx).await;
                            }
                            Err(e) => {
                                let _ = outbound_tx.send(ServerMessage::Error {
                                    message: format!("invalid message: {e}"),
                                });
                            }
                        }
                    }
                    Message::Close(_) => break,
                    // Pong / Ping / Binary all count as "still alive" via the
                    // last_seen update above; we don't otherwise act on them.
                    _ => {}
                }
            }
            _ = heartbeat.tick() => {
                if last_seen.elapsed() > IDLE_TIMEOUT {
                    tracing::info!(user_id, "ws idle timeout; closing");
                    break;
                }
                // Nudge the client to prove liveness. We route through the
                // same outbound channel the writer drains, so we don't need
                // to share the sink.
                let _ = outbound_tx.send(ServerMessage::Pong);
            }
        }
    }

    // Cleanup: abort writer and await so hub_rx is dropped, marking hub_tx as closed
    writer_task.abort();
    let _ = writer_task.await;
    hub.unregister(user_id);
}

/// Authenticate the first message. Returns (`user_id`, `user_pid`) or None on failure.
async fn authenticate(
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    ws_rx: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Option<(i32, uuid::Uuid)> {
    use crate::db::schema::{sessions, users};
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(AUTH_TIMEOUT_SECS),
        ws_rx.next(),
    );

    let Ok(Some(Ok(Message::Text(frame)))) = timeout.await else {
        tracing::warn!("WS auth failed: timeout or invalid frame");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "auth timeout or invalid frame".to_string(),
            },
        )
        .await;
        return None;
    };

    let parsed: Result<ClientMessage, _> = serde_json::from_str(&frame);
    let Ok(ClientMessage::Auth { token }) = parsed else {
        tracing::warn!("WS auth failed: first message was not auth");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "first message must be auth".to_string(),
            },
        )
        .await;
        return None;
    };

    // Validate the bearer token using existing auth infrastructure
    let hashed = hash_session_token(&token);

    let Ok(mut conn) = crate::db::conn().await else {
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "internal error".to_string(),
            },
        )
        .await;
        return None;
    };

    let row: Option<(i32, uuid::Uuid)> = if let Ok(row) = sessions::table
        .inner_join(users::table.on(users::id.eq(sessions::user_id)))
        .filter(sessions::token.eq(&hashed))
        .filter(sessions::expires_at.gt(chrono::Utc::now()))
        .select((users::id, users::pid))
        .first::<(i32, uuid::Uuid)>(&mut conn)
        .await
        .optional()
    {
        row
    } else {
        tracing::warn!("WS auth failed: database query error");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "internal error".to_string(),
            },
        )
        .await;
        return None;
    };

    if let Some((uid, pid)) = row {
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthOk {
                user_id: uid.to_string(),
            },
        )
        .await;
        Some((uid, pid))
    } else {
        tracing::warn!("WS auth failed: invalid or expired token");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "invalid token".to_string(),
            },
        )
        .await;
        None
    }
}

/// Process a single client message.
async fn handle_client_message(
    msg: ClientMessage,
    user_id: i32,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    match msg {
        ClientMessage::Auth { .. } => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "already authenticated".to_string(),
            });
        }
        ClientMessage::Send {
            conversation_id,
            body,
            reply_to_id,
            client_id,
        } => {
            handle_send(
                user_id,
                conversation_id,
                &body,
                "text",
                reply_to_id,
                client_id,
                hub,
                outbound_tx,
            )
            .await;
        }
        ClientMessage::Edit { message_id, body } => {
            handle_edit(user_id, message_id, &body, hub, outbound_tx).await;
        }
        ClientMessage::Delete { message_id } => {
            handle_delete(user_id, message_id, hub, outbound_tx).await;
        }
        ClientMessage::React { message_id, emoji } => {
            if emoji.chars().count() > 32 {
                let _ = outbound_tx.send(ServerMessage::Error {
                    message: "emoji too long".to_string(),
                });
            } else {
                handle_react(user_id, message_id, &emoji, hub, outbound_tx).await;
            }
        }
        ClientMessage::Read {
            conversation_id,
            message_id,
        } => {
            handle_read(user_id, conversation_id, message_id, hub).await;
        }
        ClientMessage::Typing {
            conversation_id,
            is_typing,
        } => {
            handle_typing(user_id, conversation_id, is_typing, hub).await;
        }
        ClientMessage::History {
            conversation_id,
            before,
            limit,
        } => {
            handle_history(user_id, conversation_id, before, limit, outbound_tx).await;
        }
        ClientMessage::ListConversations => {
            let conv_list = conversations::list_for_user(user_id)
                .await
                .unwrap_or_default();
            let _ = outbound_tx.send(ServerMessage::Conversations {
                conversations: conv_list,
            });
        }
        ClientMessage::Ping => {
            let _ = outbound_tx.send(ServerMessage::Pong);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_send(
    user_id: i32,
    conversation_id: uuid::Uuid,
    body: &str,
    kind: &str,
    reply_to_id: Option<uuid::Uuid>,
    client_id: Option<String>,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    // Validate body
    if body.trim().is_empty() {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "message body cannot be empty".to_string(),
        });
        return;
    }

    if body.len() > 10_000 {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "message body too long (max 10KB)".to_string(),
        });
        return;
    }

    if let Some(ref cid) = client_id {
        if cid.len() > 64 {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "client_id too long (max 64 chars)".to_string(),
            });
            return;
        }
    }

    // Only text messages are supported
    if kind != "text" {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "only text messages are supported".to_string(),
        });
        return;
    }

    // Verify membership
    if !conversations::is_member(conversation_id, user_id)
        .await
        .unwrap_or(false)
    {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "not a member of this conversation".to_string(),
        });
        return;
    }

    // Block check for DM conversations
    if matches!(is_blocked_in_dm(conversation_id, user_id).await, Ok(true)) {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "blocked".to_string(),
        });
        return;
    }

    match chat_messages::create_message(
        conversation_id,
        user_id,
        body,
        kind,
        reply_to_id,
        client_id,
    )
    .await
    {
        Ok((_msg, payload, created)) => {
            let server_msg = ServerMessage::Message {
                msg: Box::new(payload),
            };

            if created {
                tracing::info!(
                    conversation_id = %conversation_id,
                    sender_id = user_id,
                    "message_sent"
                );

                let members = conversations::member_user_ids(conversation_id)
                    .await
                    .unwrap_or_default();

                // Broadcast to all members (including sender for confirmation)
                hub.broadcast(&members, &server_msg);

                // Push to all non-sender members; client-side ActiveChat check
                // suppresses the notification when the user is viewing this chat.
                let push_targets: Vec<i32> = members
                    .iter()
                    .copied()
                    .filter(|&id| id != user_id)
                    .collect();
                if !push_targets.is_empty() {
                    let msg_body = body.to_string();
                    tokio::spawn(async move {
                        super::push::notify_push(push_targets, conversation_id, user_id, &msg_body)
                            .await;
                    });
                }
            } else {
                // Dedup retry — confirm to sender only, no broadcast/push
                let _ = outbound_tx.send(server_msg);
            }
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("send failed: {e}"),
            });
        }
    }
}

/// Check that a message exists and the user is a member of its conversation.
/// Returns the conversation ID on success, or sends an error and returns None.
async fn verify_message_membership(
    message_id: uuid::Uuid,
    user_id: i32,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) -> Option<uuid::Uuid> {
    match chat_messages::message_conversation_id(message_id).await {
        Ok(Some(cid))
            if conversations::is_member(cid, user_id)
                .await
                .unwrap_or(false) =>
        {
            Some(cid)
        }
        Ok(Some(_)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".into(),
            });
            None
        }
        Ok(None) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "message not found".into(),
            });
            None
        }
        Err(_) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "internal error".into(),
            });
            None
        }
    }
}

async fn handle_edit(
    user_id: i32,
    message_id: uuid::Uuid,
    body: &str,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    if body.trim().is_empty() {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "message body cannot be empty".to_string(),
        });
        return;
    }
    if body.len() > 10_000 {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "message body too long (max 10KB)".to_string(),
        });
        return;
    }

    let Some(_) = verify_message_membership(message_id, user_id, outbound_tx).await else {
        return;
    };

    match chat_messages::edit_message(message_id, user_id, body).await {
        Ok(msg) => {
            let members = conversations::member_user_ids(msg.conversation_id)
                .await
                .unwrap_or_default();
            let server_msg = ServerMessage::Edited {
                message_id: msg.id,
                conversation_id: msg.conversation_id,
                body: msg.body,
                edited_at: msg.edited_at.map_or_else(String::new, |t| t.to_rfc3339()),
            };
            hub.broadcast(&members, &server_msg);
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("edit failed: {e}"),
            });
        }
    }
}

async fn handle_delete(
    user_id: i32,
    message_id: uuid::Uuid,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    let Some(_) = verify_message_membership(message_id, user_id, outbound_tx).await else {
        return;
    };

    match chat_messages::delete_message(message_id, user_id).await {
        Ok(msg) => {
            let members = conversations::member_user_ids(msg.conversation_id)
                .await
                .unwrap_or_default();
            let server_msg = ServerMessage::Deleted {
                message_id: msg.id,
                conversation_id: msg.conversation_id,
            };
            hub.broadcast(&members, &server_msg);
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("delete failed: {e}"),
            });
        }
    }
}

async fn handle_react(
    user_id: i32,
    message_id: uuid::Uuid,
    emoji: &str,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    let Some(_) = verify_message_membership(message_id, user_id, outbound_tx).await else {
        return;
    };

    match chat_messages::toggle_reaction(message_id, user_id, emoji).await {
        Ok((added, msg)) => {
            let members = conversations::member_user_ids(msg.conversation_id)
                .await
                .unwrap_or_default();

            let (sender_name, sender_avatar) = resolve_sender_for_reaction(user_id).await;
            let server_msg = ServerMessage::Reaction {
                message_id: msg.id,
                conversation_id: msg.conversation_id,
                emoji: emoji.to_string(),
                user_id,
                added,
                sender_name,
                sender_avatar,
            };
            hub.broadcast(&members, &server_msg);
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("react failed: {e}"),
            });
        }
    }
}

async fn resolve_sender_for_reaction(user_id: i32) -> (String, Option<String>) {
    use crate::db::schema::{profiles, users};
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let Ok(mut conn) = crate::db::conn().await else {
        return ("Unknown".into(), None);
    };
    let Ok((name, avatar)) = profiles::table
        .inner_join(users::table.on(users::id.eq(profiles::user_id)))
        .filter(users::id.eq(user_id))
        .select((profiles::name, profiles::profile_picture))
        .first::<(String, Option<String>)>(&mut conn)
        .await
    else {
        return ("Unknown".into(), None);
    };
    let avatar_url = avatar
        .as_ref()
        .and_then(|f| crate::api::imgproxy_signing::signed_avatar_url(f));
    (name, avatar_url)
}

async fn handle_read(
    user_id: i32,
    conversation_id: uuid::Uuid,
    message_id: uuid::Uuid,
    hub: &ChatHub,
) {
    if !conversations::is_member(conversation_id, user_id)
        .await
        .unwrap_or(false)
    {
        return;
    }

    if chat_messages::mark_read(conversation_id, user_id, message_id)
        .await
        .is_ok()
    {
        let members = conversations::member_user_ids(conversation_id)
            .await
            .unwrap_or_default();
        let server_msg = ServerMessage::ReadReceipt {
            conversation_id,
            user_id,
            message_id,
        };
        hub.broadcast(&members, &server_msg);
    }
}

async fn handle_typing(user_id: i32, conversation_id: uuid::Uuid, is_typing: bool, hub: &ChatHub) {
    if !conversations::is_member(conversation_id, user_id)
        .await
        .unwrap_or(false)
    {
        return;
    }

    let members = conversations::member_user_ids(conversation_id)
        .await
        .unwrap_or_default();
    let server_msg = ServerMessage::Typing {
        conversation_id,
        user_id,
        is_typing,
    };
    // Send to all members except the typer
    let others: Vec<i32> = members.into_iter().filter(|id| *id != user_id).collect();
    hub.broadcast(&others, &server_msg);
}

async fn handle_history(
    user_id: i32,
    conversation_id: uuid::Uuid,
    before: Option<uuid::Uuid>,
    limit: Option<i64>,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    if !conversations::is_member(conversation_id, user_id)
        .await
        .unwrap_or(false)
    {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "not a member of this conversation".to_string(),
        });
        return;
    }

    let limit = limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);

    match chat_messages::load_history(conversation_id, before, limit, user_id).await {
        Ok((messages, has_more)) => {
            let _ = outbound_tx.send(ServerMessage::HistoryResponse {
                conversation_id,
                messages,
                has_more,
            });
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("history failed: {e}"),
            });
        }
    }
}

/// Check if the sender is blocked in a DM conversation.
/// Returns true if the other party has blocked the sender.
#[allow(clippy::similar_names)]
async fn is_blocked_in_dm(
    conversation_id: uuid::Uuid,
    sender_user_id: i32,
) -> Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    // Only check DM conversations
    let conv: Option<crate::db::models::conversations::Conversation> = conversations_table::table
        .find(conversation_id)
        .first(&mut conn)
        .await
        .optional()?;

    let Some(conv) = conv else {
        return Ok(false);
    };
    if conv.kind != "dm" {
        return Ok(false);
    }

    // Get both users' profile IDs
    let other_user_id = if conv.user_low_id == Some(sender_user_id) {
        conv.user_high_id
    } else {
        conv.user_low_id
    };

    let Some(other_user_id) = other_user_id else {
        return Ok(false);
    };

    // Resolve both profile IDs
    let sender_profile: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(sender_user_id))
        .select(profiles::id)
        .first(&mut conn)
        .await
        .optional()?;
    let other_profile: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(other_user_id))
        .select(profiles::id)
        .first(&mut conn)
        .await
        .optional()?;

    let (Some(sender_pid), Some(other_pid)) = (sender_profile, other_profile) else {
        return Ok(false);
    };

    // Check if either party has blocked the other
    let count = profile_blocks::table
        .filter(
            profile_blocks::blocker_id
                .eq(sender_pid)
                .and(profile_blocks::blocked_id.eq(other_pid))
                .or(profile_blocks::blocker_id
                    .eq(other_pid)
                    .and(profile_blocks::blocked_id.eq(sender_pid))),
        )
        .count()
        .get_result::<i64>(&mut conn)
        .await?;

    Ok(count > 0)
}

async fn send_json(
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<(), ()> {
    match serde_json::to_string(msg) {
        Ok(json) => ws_tx.send(Message::text(json)).await.map_err(|_| ()),
        Err(_) => Err(()),
    }
}
