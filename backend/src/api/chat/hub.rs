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
    /// Returns a receiver for hub-dispatched messages.
    pub fn register(&self, user_id: i32) -> mpsc::UnboundedReceiver<ServerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections.entry(user_id).or_default().push(tx);
        rx
    }

    /// Remove closed senders for the given user.
    /// Uses `remove_if_mut` for atomic retain-and-remove (no race with `register`).
    pub fn unregister(&self, user_id: i32) {
        self.connections.remove_if_mut(&user_id, |_, senders| {
            senders.retain(|s| !s.is_closed());
            senders.is_empty()
        });
    }

    /// Send a message to all connections of the listed user IDs.
    /// Prunes closed senders on the fly.
    pub fn broadcast(&self, user_ids: &[i32], msg: &ServerMessage) {
        for uid in user_ids {
            self.connections.remove_if_mut(uid, |_, senders| {
                senders.retain(|s| !s.is_closed());
                for sender in senders.iter() {
                    let _ = sender.send(msg.clone());
                }
                senders.is_empty()
            });
        }
    }

    /// Send a message to a single user (all their connections).
    pub fn send_to_user(&self, user_id: i32, msg: &ServerMessage) {
        self.connections.remove_if_mut(&user_id, |_, senders| {
            senders.retain(|s| !s.is_closed());
            for sender in senders.iter() {
                let _ = sender.send(msg.clone());
            }
            senders.is_empty()
        });
    }

    /// Check whether a user has at least one active connection.
    /// Prunes closed senders before checking.
    pub fn is_online(&self, user_id: i32) -> bool {
        let mut online = false;
        self.connections.remove_if_mut(&user_id, |_, senders| {
            senders.retain(|s| !s.is_closed());
            online = !senders.is_empty();
            senders.is_empty()
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
}
