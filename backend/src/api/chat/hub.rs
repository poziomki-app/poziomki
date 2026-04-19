use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::protocol::ServerMessage;

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

/// In-memory registry of active WebSocket connections, keyed by `users.id`.
///
/// A single user may have multiple connections (e.g. phone + tablet),
/// so we store a `Vec<ConnHandle>` per user.
#[derive(Debug, Clone)]
pub struct ChatHub {
    connections: Arc<DashMap<i32, Vec<ConnHandle>>>,
}

impl ChatHub {
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
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

    /// Remove closed senders for the given user.
    /// Uses `remove_if_mut` for atomic retain-and-remove (no race with `register`).
    pub fn unregister(&self, user_id: i32) {
        self.connections.remove_if_mut(&user_id, |_, handles| {
            handles.retain(|h| !h.sender.is_closed());
            handles.is_empty()
        });
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
    }
}
