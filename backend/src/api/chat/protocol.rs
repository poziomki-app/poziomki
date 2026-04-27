use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Client → Server
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    Auth {
        token: String,
    },
    Send {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        body: String,
        #[serde(rename = "replyToId")]
        reply_to_id: Option<Uuid>,
        #[serde(rename = "clientId")]
        client_id: Option<String>,
    },
    Edit {
        #[serde(rename = "messageId")]
        message_id: Uuid,
        body: String,
    },
    Delete {
        #[serde(rename = "messageId")]
        message_id: Uuid,
    },
    React {
        #[serde(rename = "messageId")]
        message_id: Uuid,
        emoji: String,
    },
    Read {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        #[serde(rename = "messageId")]
        message_id: Uuid,
    },
    Typing {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        #[serde(rename = "isTyping")]
        is_typing: bool,
    },
    History {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        before: Option<Uuid>,
        limit: Option<i64>,
    },
    ListConversations,
    Ping,
}

// ---------------------------------------------------------------------------
// Server → Client
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    AuthOk {
        #[serde(rename = "userId")]
        user_id: String,
    },
    AuthError {
        message: String,
    },
    Message {
        #[serde(flatten)]
        msg: Box<MessagePayload>,
    },
    Edited {
        #[serde(rename = "messageId")]
        message_id: Uuid,
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        body: String,
        #[serde(rename = "editedAt")]
        edited_at: String,
    },
    Deleted {
        #[serde(rename = "messageId")]
        message_id: Uuid,
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
    },
    Reaction {
        #[serde(rename = "messageId")]
        message_id: Uuid,
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        emoji: String,
        #[serde(rename = "userId")]
        user_id: i32,
        added: bool,
        #[serde(rename = "senderName")]
        sender_name: String,
        #[serde(rename = "senderAvatar")]
        sender_avatar: Option<String>,
    },
    ReadReceipt {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        #[serde(rename = "userId")]
        user_id: i32,
        #[serde(rename = "messageId")]
        message_id: Uuid,
    },
    Typing {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        #[serde(rename = "userId")]
        user_id: i32,
        #[serde(rename = "isTyping")]
        is_typing: bool,
    },
    HistoryResponse {
        #[serde(rename = "conversationId")]
        conversation_id: Uuid,
        messages: Vec<MessagePayload>,
        #[serde(rename = "hasMore")]
        has_more: bool,
    },
    Conversations {
        conversations: Vec<ConversationPayload>,
    },
    Pong,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: i32,
    pub sender_pid: Option<String>,
    pub sender_name: String,
    pub sender_avatar: Option<String>,
    pub sender_status: Option<String>,
    pub body: String,
    pub kind: String,
    pub reply_to: Option<ReplyPayload>,
    pub reactions: Vec<ReactionPayload>,
    pub client_id: Option<String>,
    pub is_mine: bool,
    pub is_edited: bool,
    pub created_at: String,
    /// Bielik-Guard verdict: `None` = not yet scanned (clients render
    /// as allow), `"allow"` / `"flag"` / `"block"` once scanned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation_verdict: Option<String>,
    /// Categories above the flag threshold. Always present (possibly
    /// empty) so clients don't need to default-construct.
    #[serde(default)]
    pub moderation_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyPayload {
    pub message_id: Uuid,
    pub sender_name: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionPayload {
    pub emoji: String,
    pub count: i64,
    pub reacted_by_me: bool,
    pub user_ids: Vec<i32>,
    pub sender_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPayload {
    pub id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub is_direct: bool,
    pub direct_user_id: Option<String>,
    pub direct_user_pid: Option<String>,
    pub direct_user_name: Option<String>,
    pub direct_user_avatar: Option<String>,
    pub direct_user_status: Option<String>,
    pub unread_count: i64,
    pub latest_message: Option<String>,
    pub latest_timestamp: Option<String>,
    pub latest_message_is_mine: bool,
    pub latest_sender_name: Option<String>,
    pub is_blocked: bool,
}
