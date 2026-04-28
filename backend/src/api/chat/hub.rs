use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::protocol::ServerMessage;

/// How long after the last typing-true frame the server treats a user
/// as still typing. Picked to match the keystroke cadence of mobile
/// keyboards (clients should re-emit `Typing(true)` every few seconds
/// while still typing). Exposed publicly so the WS handler can echo
/// the same TTL to clients via `expires_in_ms`, removing the need for
/// a hardcoded client-side fallback.
pub const TYPING_TTL: std::time::Duration = std::time::Duration::from_millis(6_000);

/// How often the janitor sweeps the typing map for expired entries.
/// Coarser than the TTL so a single tokio task can comfortably handle
/// the chat-wide load; the worst-case ghost-typing window is
/// `TYPING_TTL + JANITOR_INTERVAL`.
const JANITOR_INTERVAL: std::time::Duration = std::time::Duration::from_millis(2_000);

/// A handle to one WebSocket connection:
/// * `sender` — outbound message channel (hub → writer task)
/// * `kill` — force-disconnect signal (hub → reader task). When the
///   reader receives on this channel it breaks its loop, which aborts
///   the writer task, which unregisters the connection.
#[derive(Debug)]
struct ConnHandle {
    sender: mpsc::UnboundedSender<ServerMessage>,
    kill: mpsc::UnboundedSender<()>,
}

/// Tracked typing state per `(user_id, conversation_id)`. We store the
/// full member list so the janitor can fan out a stop-typing event
/// without re-querying the DB; the WS handler captures it when the
/// user originally signalled typing=true.
#[derive(Debug, Clone)]
struct TypingState {
    expires_at: std::time::Instant,
    members: Vec<i32>,
}

/// In-memory registry of active WebSocket connections, keyed by `users.id`.
///
/// A single user may have multiple connections (e.g. phone + tablet),
/// so we store a `Vec<ConnHandle>` per user.
#[derive(Debug, Clone)]
pub struct ChatHub {
    connections: Arc<DashMap<i32, Vec<ConnHandle>>>,
    /// `(user_id, conversation_id) → typing state`. The janitor
    /// scans this map every `JANITOR_INTERVAL` and broadcasts
    /// `Typing(false)` for any entry past its deadline. Lets us
    /// clear ghost typers when a client crashes mid-typing.
    typing: Arc<DashMap<(i32, Uuid), TypingState>>,
}

impl ChatHub {
    #[must_use]
    pub fn new() -> Self {
        let hub = Self {
            connections: Arc::new(DashMap::new()),
            typing: Arc::new(DashMap::new()),
        };
        hub.spawn_typing_janitor();
        hub
    }

    /// Register a new connection for the given user.
    ///
    /// Returns `(messages, kill)` — the writer task drains `messages`,
    /// the reader task selects on `kill` to break out when the hub
    /// signals a force-disconnect.
    pub fn register(
        &self,
        user_id: i32,
    ) -> (
        mpsc::UnboundedReceiver<ServerMessage>,
        mpsc::UnboundedReceiver<()>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (kill_tx, kill_rx) = mpsc::unbounded_channel();
        self.connections
            .entry(user_id)
            .or_default()
            .push(ConnHandle {
                sender: tx,
                kill: kill_tx,
            });
        (rx, kill_rx)
    }

    /// Remove closed senders for the given user. If this empties the
    /// user's connection list, also clear their typing state and emit
    /// a stop-typing fanout so peers don't see a ghost typer for
    /// `TYPING_TTL` seconds after a crash.
    pub fn unregister(&self, user_id: i32) {
        let mut became_empty = false;
        self.connections.remove_if_mut(&user_id, |_, handles| {
            handles.retain(|h| !h.sender.is_closed());
            became_empty = handles.is_empty();
            became_empty
        });
        if became_empty {
            self.flush_typing_for_user(user_id);
        }
    }

    /// Send a message to all connections of the listed user IDs.
    /// Prunes closed senders on the fly.
    pub fn broadcast(&self, user_ids: &[i32], msg: &ServerMessage) {
        for uid in user_ids {
            self.connections.remove_if_mut(uid, |_, handles| {
                handles.retain(|h| !h.sender.is_closed());
                for handle in handles.iter() {
                    let _ = handle.sender.send(msg.clone());
                }
                handles.is_empty()
            });
        }
    }

    /// Check whether a user has at least one active connection.
    /// Prunes closed senders before checking.
    pub fn is_online(&self, user_id: i32) -> bool {
        let mut online = false;
        self.connections.remove_if_mut(&user_id, |_, handles| {
            handles.retain(|h| !h.sender.is_closed());
            online = !handles.is_empty();
            handles.is_empty()
        });
        online
    }

    /// Return the list of user IDs that have NO active connections
    /// from the given set. Useful for push notification targeting.
    pub fn offline_users(&self, user_ids: &[i32]) -> Vec<i32> {
        user_ids
            .iter()
            .filter(|uid| !self.is_online(**uid))
            .copied()
            .collect()
    }

    /// Force every live WebSocket for the given user to drop.
    ///
    /// Used by `sign_out_all` and the admin ban endpoint so a
    /// revoked session can't keep a previously-authenticated WS
    /// connection alive until the next reconnect. Each reader
    /// task receives the kill signal, breaks its loop, and the
    /// normal teardown path (writer abort + `unregister`) runs.
    pub fn disconnect_user(&self, user_id: i32) {
        if let Some((_, handles)) = self.connections.remove(&user_id) {
            for handle in handles {
                let _ = handle.kill.send(());
            }
        }
        self.flush_typing_for_user(user_id);
    }

    /// Note that `user_id` is currently typing in `conversation_id`.
    /// Resets the TTL on every call. The WS handler is responsible for
    /// providing the conversation member list once at typing-true; the
    /// janitor keeps reusing it without re-querying.
    pub fn note_typing(&self, user_id: i32, conversation_id: Uuid) {
        let now = std::time::Instant::now();
        let key = (user_id, conversation_id);
        self.typing
            .entry(key)
            .and_modify(|s| s.expires_at = now + TYPING_TTL)
            .or_insert_with(|| TypingState {
                expires_at: now + TYPING_TTL,
                members: Vec::new(),
            });
    }

    /// Cache the conversation members for a typing entry so the janitor
    /// can broadcast the eventual Typing(false) without DB access.
    pub fn note_typing_members(&self, user_id: i32, conversation_id: Uuid, members: Vec<i32>) {
        let key = (user_id, conversation_id);
        if let Some(mut entry) = self.typing.get_mut(&key) {
            entry.members = members;
        }
    }

    /// Drop the typing entry for `(user, conv)`. Caller still
    /// broadcasts the explicit `Typing(false)`.
    pub fn clear_typing(&self, user_id: i32, conversation_id: Uuid) {
        self.typing.remove(&(user_id, conversation_id));
    }

    /// Drop every typing entry for `user_id` and broadcast
    /// `Typing(false)` to each affected conversation. Called from the
    /// disconnect path so peers stop seeing "X is typing…" within ~the
    /// next janitor tick rather than the full TTL.
    fn flush_typing_for_user(&self, user_id: i32) {
        let mut to_clear: Vec<(Uuid, Vec<i32>)> = Vec::new();
        self.typing.retain(|(uid, conv_id), state| {
            if *uid == user_id {
                to_clear.push((*conv_id, state.members.clone()));
                false
            } else {
                true
            }
        });
        for (conv_id, members) in to_clear {
            let others: Vec<i32> = members.into_iter().filter(|id| *id != user_id).collect();
            if others.is_empty() {
                continue;
            }
            self.broadcast(
                &others,
                &ServerMessage::Typing {
                    conversation_id: conv_id,
                    user_id,
                    is_typing: false,
                    expires_in_ms: None,
                },
            );
        }
    }

    /// Spawn the periodic sweeper that fires `Typing(false)` for any
    /// entry whose deadline has passed. Crash-resilient — the task
    /// holds an `Arc` clone of the typing map and connections so the
    /// hub itself can be cloned freely.
    fn spawn_typing_janitor(&self) {
        let typing = Arc::clone(&self.typing);
        let connections = Arc::clone(&self.connections);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(JANITOR_INTERVAL);
            // Skip the first immediate tick.
            interval.tick().await;
            loop {
                interval.tick().await;
                let now = std::time::Instant::now();
                let mut expired: Vec<((i32, Uuid), Vec<i32>)> = Vec::new();
                typing.retain(|key, state| {
                    if state.expires_at <= now {
                        expired.push((*key, state.members.clone()));
                        false
                    } else {
                        true
                    }
                });
                for ((user_id, conv_id), members) in expired {
                    let others: Vec<i32> =
                        members.into_iter().filter(|id| *id != user_id).collect();
                    let msg = ServerMessage::Typing {
                        conversation_id: conv_id,
                        user_id,
                        is_typing: false,
                        expires_in_ms: None,
                    };
                    for uid in &others {
                        connections.remove_if_mut(uid, |_, handles| {
                            handles.retain(|h| !h.sender.is_closed());
                            for handle in handles.iter() {
                                let _ = handle.sender.send(msg.clone());
                            }
                            handles.is_empty()
                        });
                    }
                }
            }
        });
    }
}
