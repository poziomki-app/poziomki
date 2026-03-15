use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::protocol::ServerMessage;

/// A handle to one WebSocket connection's outbound channel.
type Sender = mpsc::UnboundedSender<ServerMessage>;

/// In-memory registry of active WebSocket connections, keyed by `users.id`.
///
/// A single user may have multiple connections (e.g. phone + tablet),
/// so we store a `Vec<Sender>` per user.
#[derive(Debug, Clone)]
pub struct ChatHub {
    connections: Arc<DashMap<i32, Vec<Sender>>>,
}

impl ChatHub {
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Register a new connection for the given user.
    /// Returns the `(Sender, Receiver)` pair. The caller must pass the `Sender`
    /// back to [`unregister`] so the connection can be cleaned up.
    pub fn register(&self, user_id: i32) -> (Sender, mpsc::UnboundedReceiver<ServerMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections
            .entry(user_id)
            .or_default()
            .push(tx.clone());
        (tx, rx)
    }

    /// Remove closed senders for the given user.
    pub fn unregister(&self, user_id: i32) {
        if let Some(mut entry) = self.connections.get_mut(&user_id) {
            entry.retain(|s| !s.is_closed());
            if entry.is_empty() {
                drop(entry);
                self.connections.remove(&user_id);
            }
        }
    }

    /// Send a message to all connections of the listed user IDs.
    /// Prunes closed senders on the fly.
    pub fn broadcast(&self, user_ids: &[i32], msg: &ServerMessage) {
        for uid in user_ids {
            if let Some(mut senders) = self.connections.get_mut(uid) {
                senders.retain(|s| !s.is_closed());
                for sender in senders.value() {
                    let _ = sender.send(msg.clone());
                }
                if senders.is_empty() {
                    drop(senders);
                    self.connections.remove(uid);
                }
            }
        }
    }

    /// Send a message to a single user (all their connections).
    pub fn send_to_user(&self, user_id: i32, msg: &ServerMessage) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            senders.retain(|s| !s.is_closed());
            for sender in senders.value() {
                let _ = sender.send(msg.clone());
            }
            if senders.is_empty() {
                drop(senders);
                self.connections.remove(&user_id);
            }
        }
    }

    /// Check whether a user has at least one active connection.
    /// Prunes closed senders before checking.
    pub fn is_online(&self, user_id: i32) -> bool {
        if let Some(mut entry) = self.connections.get_mut(&user_id) {
            entry.retain(|s| !s.is_closed());
            if entry.is_empty() {
                drop(entry);
                self.connections.remove(&user_id);
                return false;
            }
            true
        } else {
            false
        }
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
}
