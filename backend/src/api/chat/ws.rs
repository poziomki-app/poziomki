use axum::extract::ws::{Message, WebSocket};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use super::conversations;
use super::hub::ChatHub;
use super::messages as chat_messages;
use super::protocol::{ClientMessage, ServerMessage};
use crate::api::state::hash_session_token;
use crate::db;
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

/// Authenticated-viewer state carried through the socket lifetime.
#[derive(Clone, Copy)]
struct SocketViewer {
    user_id: i32,
    is_review_stub: bool,
}

impl From<SocketViewer> for db::DbViewer {
    fn from(v: SocketViewer) -> Self {
        Self {
            user_id: v.user_id,
            is_review_stub: v.is_review_stub,
        }
    }
}

/// Handle a WebSocket connection after Axum upgrade.
pub async fn handle_socket(socket: WebSocket, hub: ChatHub) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // --- Step 1: Authenticate ---
    let Some((viewer, _user_pid)) = authenticate(&mut ws_tx, &mut ws_rx).await else {
        return;
    };
    let user_id = viewer.user_id;
    let viewer_is_stub = viewer.is_review_stub;

    // --- Step 2: Register in hub ---
    let (mut hub_rx, mut kill_rx) = hub.register(user_id);
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<ServerMessage>();
    // Separate channel for WS protocol-level Ping frames. Using the standard
    // control frame (rather than an app-level Pong JSON) means the client's
    // WebSocket stack auto-responds at the framing layer and no app code
    // sees the heartbeat.
    let (ping_tx, mut ping_rx) = mpsc::unbounded_channel::<()>();

    // Send initial conversation list
    let conv_list = db::with_viewer_tx(viewer.into(), move |conn| {
        async move {
            conversations::list_for_user(conn, user_id, viewer_is_stub)
                .await
                .map_err(into_diesel)
        }
        .scope_boxed()
    })
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
    // Spawn a writer task that drains hub messages, outbound messages, and
    // heartbeat pings. It owns `ws_tx` so all writes go through a single
    // place.
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
                Some(()) = ping_rx.recv() => {
                    if ws_tx.send(Message::Ping(Vec::new().into())).await.is_err() {
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
            // Force-disconnect from the hub (sign_out_all / admin ban).
            // We break out so the cleanup path below aborts the
            // writer task and unregisters the connection, the same
            // way an idle timeout or client close would.
            Some(()) = kill_rx.recv() => {
                tracing::info!(user_id, "ws force-disconnected by hub");
                break;
            }
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
                                handle_client_message(client_msg, viewer, &hub, &outbound_tx).await;
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
                // Signal the writer task to send a WS-level Ping. The
                // client's WebSocket library replies with Pong at the
                // framing layer, which resets `last_seen` via the reader
                // arm above without surfacing to app code.
                let _ = ping_tx.send(());
            }
        }
    }

    // Cleanup: abort writer and await so hub_rx is dropped, marking hub_tx as closed
    writer_task.abort();
    let _ = writer_task.await;
    hub.unregister(user_id);
}

/// Convert an `AppError` from a library helper into a `diesel::result::Error`
/// that rolls back the transaction.
///
/// `AppError::Any` carries diesel / Postgres / pool errors — those collapse
/// to an opaque `RollbackTransaction` so raw database error text (column
/// names, constraint names, etc.) never leaks over the WebSocket. The
/// `Message` / `Validation` variants are intentional, application-level
/// strings and are safe to surface to clients, so they flow through
/// `QueryBuilderError` and render via `format!("{e}")` at the outer match.
fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

/// Authenticate the first message. Returns a `SocketViewer` + user pid, or
/// None on failure. Routes through `app.resolve_session` so it works once
/// Tier-A RLS lands on the sessions table.
async fn authenticate(
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    ws_rx: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Option<(SocketViewer, uuid::Uuid)> {
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

    let Ok(session) = db::resolve_session(&mut conn, &hashed).await else {
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

    let Some(session) = session else {
        tracing::warn!("WS auth failed: invalid token");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "invalid token".to_string(),
            },
        )
        .await;
        return None;
    };

    if session.expires_at <= chrono::Utc::now() {
        tracing::warn!("WS auth failed: session expired");
        let _ = send_json(
            ws_tx,
            &ServerMessage::AuthError {
                message: "invalid token".to_string(),
            },
        )
        .await;
        return None;
    }

    let viewer = SocketViewer {
        user_id: session.user_id,
        is_review_stub: session.is_review_stub,
    };

    let _ = send_json(
        ws_tx,
        &ServerMessage::AuthOk {
            user_id: viewer.user_id.to_string(),
        },
    )
    .await;
    Some((viewer, session.user_pid))
}

/// Process a single client message.
async fn handle_client_message(
    msg: ClientMessage,
    viewer: SocketViewer,
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
                viewer,
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
            handle_edit(viewer, message_id, &body, hub, outbound_tx).await;
        }
        ClientMessage::Delete { message_id } => {
            handle_delete(viewer, message_id, hub, outbound_tx).await;
        }
        ClientMessage::React { message_id, emoji } => {
            if emoji.chars().count() > 32 {
                let _ = outbound_tx.send(ServerMessage::Error {
                    message: "emoji too long".to_string(),
                });
            } else {
                handle_react(viewer, message_id, &emoji, hub, outbound_tx).await;
            }
        }
        ClientMessage::Read {
            conversation_id,
            message_id,
        } => {
            handle_read(viewer, conversation_id, message_id, hub).await;
        }
        ClientMessage::Typing {
            conversation_id,
            is_typing,
        } => {
            handle_typing(viewer, conversation_id, is_typing, hub).await;
        }
        ClientMessage::History {
            conversation_id,
            before,
            limit,
        } => {
            handle_history(viewer, conversation_id, before, limit, outbound_tx).await;
        }
        ClientMessage::ListConversations => {
            let user_id = viewer.user_id;
            let viewer_is_stub = viewer.is_review_stub;
            let conv_list = db::with_viewer_tx(viewer.into(), move |conn| {
                async move {
                    conversations::list_for_user(conn, user_id, viewer_is_stub)
                        .await
                        .map_err(into_diesel)
                }
                .scope_boxed()
            })
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

/// Send outcome: Ok(message payload + recipient members) or an error message.
struct SendOutcome {
    payload: super::protocol::MessagePayload,
    members: Vec<i32>,
    created: bool,
}

#[allow(clippy::too_many_arguments, clippy::items_after_statements)]
async fn handle_send(
    viewer: SocketViewer,
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

    if kind != "text" {
        let _ = outbound_tx.send(ServerMessage::Error {
            message: "only text messages are supported".to_string(),
        });
        return;
    }

    let user_id = viewer.user_id;
    let body_owned = body.to_string();
    let kind_owned = kind.to_string();
    let client_id_for_tx = client_id.clone();

    enum SendFailure {
        NotMember,
        Blocked,
    }

    let result: std::result::Result<
        std::result::Result<SendOutcome, SendFailure>,
        diesel::result::Error,
    > = db::with_viewer_tx(viewer.into(), move |conn| {
        async move {
            if !conversations::is_member(conn, conversation_id, user_id)
                .await
                .map_err(into_diesel)?
            {
                return Ok::<std::result::Result<SendOutcome, SendFailure>, diesel::result::Error>(
                    Err(SendFailure::NotMember),
                );
            }

            if is_blocked_in_dm(conn, conversation_id, user_id)
                .await
                .map_err(into_diesel)?
            {
                return Ok(Err(SendFailure::Blocked));
            }

            // create_message does INSERT-then-payload-construction. If
            // payload construction fails post-INSERT, we MUST roll back,
            // otherwise the row sits committed without a broadcast. Route
            // any error through `?` + into_diesel so that happens — the
            // cost is that pre-write semantic errors (e.g. "reply_to does
            // not belong") also roll back (harmless) and render via the
            // `Message`/`Validation` preservation in into_diesel.
            let (_msg, payload, created) = chat_messages::create_message(
                conn,
                conversation_id,
                user_id,
                &body_owned,
                &kind_owned,
                reply_to_id,
                client_id_for_tx,
            )
            .await
            .map_err(into_diesel)?;

            let members = if created {
                conversations::member_user_ids(conn, conversation_id)
                    .await
                    .map_err(into_diesel)?
            } else {
                Vec::new()
            };

            Ok(Ok(SendOutcome {
                payload,
                members,
                created,
            }))
        }
        .scope_boxed()
    })
    .await;

    match result {
        Ok(Ok(outcome)) => {
            // NB: moderation scan is enqueued transactionally inside
            // create_message, so nothing to do here — the job is already
            // durable if this branch was reached.
            let server_msg = ServerMessage::Message {
                msg: Box::new(outcome.payload),
            };
            if outcome.created {
                tracing::info!(
                    conversation_id = %conversation_id,
                    sender_id = user_id,
                    "message_sent"
                );
                hub.broadcast(&outcome.members, &server_msg);

                let push_targets: Vec<i32> = outcome
                    .members
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
                let _ = outbound_tx.send(server_msg);
            }
        }
        Ok(Err(SendFailure::NotMember)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".to_string(),
            });
        }
        Ok(Err(SendFailure::Blocked)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "blocked".to_string(),
            });
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("send failed: {e}"),
            });
        }
    }
}

#[allow(clippy::items_after_statements)]
async fn handle_edit(
    viewer: SocketViewer,
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

    let user_id = viewer.user_id;
    let body_owned = body.to_string();

    enum Outcome {
        Edited {
            message_id: uuid::Uuid,
            conversation_id: uuid::Uuid,
            body: String,
            edited_at: Option<chrono::DateTime<chrono::Utc>>,
            members: Vec<i32>,
        },
        NotMember,
        NotFound,
        Failed(String),
    }

    let result: std::result::Result<Outcome, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                let Some(conv_id) = chat_messages::message_conversation_id(conn, message_id)
                    .await
                    .map_err(into_diesel)?
                else {
                    return Ok::<Outcome, diesel::result::Error>(Outcome::NotFound);
                };
                if !conversations::is_member(conn, conv_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok(Outcome::NotMember);
                }
                // edit_message returns AppError::Message for semantic
                // failures ("not editable", body validation) and
                // AppError::Any for DB errors. Surface the semantic text
                // via Outcome::Failed; let DB errors roll back + render
                // generically via into_diesel.
                let msg = match chat_messages::edit_message(conn, message_id, user_id, &body_owned)
                    .await
                {
                    Ok(m) => m,
                    Err(
                        crate::error::AppError::Message(m) | crate::error::AppError::Validation(m),
                    ) => return Ok(Outcome::Failed(m)),
                    Err(e) => return Err(into_diesel(e)),
                };
                let members = conversations::member_user_ids(conn, msg.conversation_id)
                    .await
                    .map_err(into_diesel)?;
                Ok(Outcome::Edited {
                    message_id: msg.id,
                    conversation_id: msg.conversation_id,
                    body: msg.body,
                    edited_at: msg.edited_at,
                    members,
                })
            }
            .scope_boxed()
        })
        .await;

    match result {
        Ok(Outcome::Edited {
            message_id,
            conversation_id,
            body,
            edited_at,
            members,
        }) => {
            // NB: moderation re-scan is enqueued transactionally inside
            // edit_message.
            let server_msg = ServerMessage::Edited {
                message_id,
                conversation_id,
                body,
                edited_at: edited_at.map_or_else(String::new, |t| t.to_rfc3339()),
            };
            hub.broadcast(&members, &server_msg);
        }
        Ok(Outcome::NotMember) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".into(),
            });
        }
        Ok(Outcome::NotFound) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "message not found".into(),
            });
        }
        Ok(Outcome::Failed(msg)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("edit failed: {msg}"),
            });
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("edit failed: {e}"),
            });
        }
    }
}

#[allow(clippy::items_after_statements)]
async fn handle_delete(
    viewer: SocketViewer,
    message_id: uuid::Uuid,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    let user_id = viewer.user_id;

    enum Outcome {
        Deleted {
            message_id: uuid::Uuid,
            conversation_id: uuid::Uuid,
            members: Vec<i32>,
        },
        NotMember,
        NotFound,
        Failed(String),
    }

    let result: std::result::Result<Outcome, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                let Some(conv_id) = chat_messages::message_conversation_id(conn, message_id)
                    .await
                    .map_err(into_diesel)?
                else {
                    return Ok::<Outcome, diesel::result::Error>(Outcome::NotFound);
                };
                if !conversations::is_member(conn, conv_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok(Outcome::NotMember);
                }
                // "message not found or already deleted" is AppError::Message;
                // DB failures are AppError::Any and get rolled back + generic.
                let msg = match chat_messages::delete_message(conn, message_id, user_id).await {
                    Ok(m) => m,
                    Err(
                        crate::error::AppError::Message(m) | crate::error::AppError::Validation(m),
                    ) => return Ok(Outcome::Failed(m)),
                    Err(e) => return Err(into_diesel(e)),
                };
                let members = conversations::member_user_ids(conn, msg.conversation_id)
                    .await
                    .map_err(into_diesel)?;
                Ok(Outcome::Deleted {
                    message_id: msg.id,
                    conversation_id: msg.conversation_id,
                    members,
                })
            }
            .scope_boxed()
        })
        .await;

    match result {
        Ok(Outcome::Deleted {
            message_id,
            conversation_id,
            members,
        }) => {
            let server_msg = ServerMessage::Deleted {
                message_id,
                conversation_id,
            };
            hub.broadcast(&members, &server_msg);
        }
        Ok(Outcome::NotMember) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".into(),
            });
        }
        Ok(Outcome::NotFound) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "message not found".into(),
            });
        }
        Ok(Outcome::Failed(msg)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("delete failed: {msg}"),
            });
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("delete failed: {e}"),
            });
        }
    }
}

#[allow(clippy::items_after_statements)]
async fn handle_react(
    viewer: SocketViewer,
    message_id: uuid::Uuid,
    emoji: &str,
    hub: &ChatHub,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    let user_id = viewer.user_id;
    let emoji_owned = emoji.to_string();

    struct ReactRow {
        message_id: uuid::Uuid,
        conversation_id: uuid::Uuid,
        added: bool,
        members: Vec<i32>,
        sender_name: String,
        sender_avatar: Option<String>,
    }

    enum Outcome {
        Reacted(ReactRow),
        NotMember,
        NotFound,
        Failed(String),
    }

    let result: std::result::Result<Outcome, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                let Some(conv_id) = chat_messages::message_conversation_id(conn, message_id)
                    .await
                    .map_err(into_diesel)?
                else {
                    return Ok::<Outcome, diesel::result::Error>(Outcome::NotFound);
                };
                if !conversations::is_member(conn, conv_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok(Outcome::NotMember);
                }
                let (added, msg) =
                    match chat_messages::toggle_reaction(conn, message_id, user_id, &emoji_owned)
                        .await
                    {
                        Ok(v) => v,
                        Err(
                            crate::error::AppError::Message(m)
                            | crate::error::AppError::Validation(m),
                        ) => return Ok(Outcome::Failed(m)),
                        Err(e) => return Err(into_diesel(e)),
                    };
                let members = conversations::member_user_ids(conn, msg.conversation_id)
                    .await
                    .map_err(into_diesel)?;
                let (sender_name, sender_avatar) = resolve_sender_for_reaction(conn, user_id).await;
                Ok(Outcome::Reacted(ReactRow {
                    message_id: msg.id,
                    conversation_id: msg.conversation_id,
                    added,
                    members,
                    sender_name,
                    sender_avatar,
                }))
            }
            .scope_boxed()
        })
        .await;

    match result {
        Ok(Outcome::Reacted(r)) => {
            let server_msg = ServerMessage::Reaction {
                message_id: r.message_id,
                conversation_id: r.conversation_id,
                emoji: emoji.to_string(),
                user_id,
                added: r.added,
                sender_name: r.sender_name,
                sender_avatar: r.sender_avatar,
            };
            hub.broadcast(&r.members, &server_msg);
        }
        Ok(Outcome::NotMember) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".into(),
            });
        }
        Ok(Outcome::NotFound) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "message not found".into(),
            });
        }
        Ok(Outcome::Failed(msg)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("react failed: {msg}"),
            });
        }
        Err(e) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("react failed: {e}"),
            });
        }
    }
}

async fn resolve_sender_for_reaction(
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> (String, Option<String>) {
    use crate::db::schema::profiles;

    // Filter on profiles.user_id directly — no users join needed.
    let Ok((name, avatar)) = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .select((profiles::name, profiles::profile_picture))
        .first::<(String, Option<String>)>(conn)
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
    viewer: SocketViewer,
    conversation_id: uuid::Uuid,
    message_id: uuid::Uuid,
    hub: &ChatHub,
) {
    let user_id = viewer.user_id;

    let members: std::result::Result<Option<Vec<i32>>, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                if !conversations::is_member(conn, conversation_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok::<Option<Vec<i32>>, diesel::result::Error>(None);
                }
                chat_messages::mark_read(conn, conversation_id, user_id, message_id)
                    .await
                    .map_err(into_diesel)?;
                let m = conversations::member_user_ids(conn, conversation_id)
                    .await
                    .map_err(into_diesel)?;
                Ok(Some(m))
            }
            .scope_boxed()
        })
        .await;

    if let Ok(Some(members)) = members {
        let server_msg = ServerMessage::ReadReceipt {
            conversation_id,
            user_id,
            message_id,
        };
        hub.broadcast(&members, &server_msg);
    }
}

async fn handle_typing(
    viewer: SocketViewer,
    conversation_id: uuid::Uuid,
    is_typing: bool,
    hub: &ChatHub,
) {
    let user_id = viewer.user_id;

    let members: std::result::Result<Option<Vec<i32>>, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                if !conversations::is_member(conn, conversation_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok::<Option<Vec<i32>>, diesel::result::Error>(None);
                }
                let m = conversations::member_user_ids(conn, conversation_id)
                    .await
                    .map_err(into_diesel)?;
                Ok(Some(m))
            }
            .scope_boxed()
        })
        .await;

    if let Ok(Some(members)) = members {
        let server_msg = ServerMessage::Typing {
            conversation_id,
            user_id,
            is_typing,
        };
        let others: Vec<i32> = members.into_iter().filter(|id| *id != user_id).collect();
        hub.broadcast(&others, &server_msg);
    }
}

#[allow(clippy::items_after_statements)]
async fn handle_history(
    viewer: SocketViewer,
    conversation_id: uuid::Uuid,
    before: Option<uuid::Uuid>,
    limit: Option<i64>,
    outbound_tx: &mpsc::UnboundedSender<ServerMessage>,
) {
    let user_id = viewer.user_id;
    let limit = limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);

    enum Outcome {
        Ok {
            messages: Vec<super::protocol::MessagePayload>,
            has_more: bool,
        },
        NotMember,
        Failed(String),
    }

    let result: std::result::Result<Outcome, diesel::result::Error> =
        db::with_viewer_tx(viewer.into(), move |conn| {
            async move {
                if !conversations::is_member(conn, conversation_id, user_id)
                    .await
                    .map_err(into_diesel)?
                {
                    return Ok::<Outcome, diesel::result::Error>(Outcome::NotMember);
                }
                // Semantic errors from load_history surface through
                // Outcome::Failed; internal DB errors get a rollback +
                // generic message.
                let (messages, has_more) = match chat_messages::load_history(
                    conn,
                    conversation_id,
                    before,
                    limit,
                    user_id,
                )
                .await
                {
                    Ok(v) => v,
                    Err(
                        crate::error::AppError::Message(m) | crate::error::AppError::Validation(m),
                    ) => return Ok(Outcome::Failed(m)),
                    Err(e) => return Err(into_diesel(e)),
                };
                Ok(Outcome::Ok { messages, has_more })
            }
            .scope_boxed()
        })
        .await;

    match result {
        Ok(Outcome::Ok { messages, has_more }) => {
            let _ = outbound_tx.send(ServerMessage::HistoryResponse {
                conversation_id,
                messages,
                has_more,
            });
        }
        Ok(Outcome::NotMember) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: "not a member of this conversation".to_string(),
            });
        }
        Ok(Outcome::Failed(msg)) => {
            let _ = outbound_tx.send(ServerMessage::Error {
                message: format!("history failed: {msg}"),
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
    conn: &mut AsyncPgConnection,
    conversation_id: uuid::Uuid,
    sender_user_id: i32,
) -> Result<bool, crate::error::AppError> {
    let conv: Option<crate::db::models::conversations::Conversation> = conversations_table::table
        .find(conversation_id)
        .first(conn)
        .await
        .optional()?;

    let Some(conv) = conv else {
        return Ok(false);
    };
    if conv.kind != "dm" {
        return Ok(false);
    }

    let other_user_id = if conv.user_low_id == Some(sender_user_id) {
        conv.user_high_id
    } else {
        conv.user_low_id
    };

    let Some(other_user_id) = other_user_id else {
        return Ok(false);
    };

    let sender_profile: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(sender_user_id))
        .select(profiles::id)
        .first(conn)
        .await
        .optional()?;
    let other_profile: Option<uuid::Uuid> = profiles::table
        .filter(profiles::user_id.eq(other_user_id))
        .select(profiles::id)
        .first(conn)
        .await
        .optional()?;

    let (Some(sender_pid), Some(other_pid)) = (sender_profile, other_profile) else {
        return Ok(false);
    };

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
        .get_result::<i64>(conn)
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
