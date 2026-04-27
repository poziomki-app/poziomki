package com.poziomki.app.chat.ws

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

val wsJson =
    Json {
        ignoreUnknownKeys = true
        classDiscriminator = "type"
        encodeDefaults = true
        explicitNulls = false
    }

// Client → Server

@Serializable
sealed interface WsClientMessage {
    @Serializable
    @SerialName("auth")
    data class Auth(
        val token: String,
    ) : WsClientMessage

    @Serializable
    @SerialName("send")
    data class Send(
        val conversationId: String,
        val body: String,
        val replyToId: String? = null,
        val clientId: String? = null,
    ) : WsClientMessage

    @Serializable
    @SerialName("edit")
    data class Edit(
        val messageId: String,
        val body: String,
    ) : WsClientMessage

    @Serializable
    @SerialName("delete")
    data class Delete(
        val messageId: String,
    ) : WsClientMessage

    @Serializable
    @SerialName("react")
    data class React(
        val messageId: String,
        val emoji: String,
    ) : WsClientMessage

    @Serializable
    @SerialName("read")
    data class Read(
        val conversationId: String,
        val messageId: String,
    ) : WsClientMessage

    @Serializable
    @SerialName("typing")
    data class Typing(
        val conversationId: String,
        val isTyping: Boolean,
    ) : WsClientMessage

    @Serializable
    @SerialName("history")
    data class History(
        val conversationId: String,
        val before: String? = null,
        val limit: Int? = null,
    ) : WsClientMessage

    @Serializable
    @SerialName("listConversations")
    data object ListConversations : WsClientMessage

    @Serializable
    @SerialName("ping")
    data object Ping : WsClientMessage
}

// Server → Client

@Serializable
sealed interface WsServerMessage {
    @Serializable
    @SerialName("authOk")
    data class AuthOk(
        val userId: String,
    ) : WsServerMessage

    @Serializable
    @SerialName("authError")
    data class AuthError(
        val message: String,
    ) : WsServerMessage

    @Serializable
    @SerialName("message")
    data class Message(
        val id: String,
        val conversationId: String,
        val senderId: Int,
        val senderPid: String? = null,
        val senderName: String,
        val senderAvatar: String? = null,
        val senderStatus: String? = null,
        val body: String,
        val kind: String = "text",
        val replyTo: WsReplyPayload? = null,
        val reactions: List<WsReactionPayload> = emptyList(),
        val clientId: String? = null,
        val isMine: Boolean = false,
        val isEdited: Boolean = false,
        val createdAt: String,
        val moderationVerdict: String? = null,
        val moderationCategories: List<String> = emptyList(),
    ) : WsServerMessage

    @Serializable
    @SerialName("edited")
    data class Edited(
        val messageId: String,
        val conversationId: String,
        val body: String,
        val editedAt: String,
    ) : WsServerMessage

    @Serializable
    @SerialName("deleted")
    data class Deleted(
        val messageId: String,
        val conversationId: String,
    ) : WsServerMessage

    @Serializable
    @SerialName("reaction")
    data class Reaction(
        val messageId: String,
        val conversationId: String,
        val emoji: String,
        val userId: Int,
        val added: Boolean,
        val senderName: String = "Unknown",
        val senderAvatar: String? = null,
    ) : WsServerMessage

    @Serializable
    @SerialName("readReceipt")
    data class ReadReceipt(
        val conversationId: String,
        val userId: Int,
        val messageId: String,
    ) : WsServerMessage

    @Serializable
    @SerialName("typing")
    data class Typing(
        val conversationId: String,
        val userId: Int,
        val isTyping: Boolean,
    ) : WsServerMessage

    @Serializable
    @SerialName("historyResponse")
    data class HistoryResponse(
        val conversationId: String,
        val messages: List<WsMessagePayload>,
        val hasMore: Boolean,
    ) : WsServerMessage

    @Serializable
    @SerialName("conversations")
    data class Conversations(
        val conversations: List<WsConversationPayload>,
    ) : WsServerMessage

    @Serializable
    @SerialName("pong")
    data object Pong : WsServerMessage

    @Serializable
    @SerialName("error")
    data class Error(
        val message: String,
    ) : WsServerMessage
}

@Serializable
data class WsMessagePayload(
    val id: String,
    val conversationId: String,
    val senderId: Int,
    val senderPid: String? = null,
    val senderName: String,
    val senderAvatar: String? = null,
    val senderStatus: String? = null,
    val body: String,
    val kind: String = "text",
    val replyTo: WsReplyPayload? = null,
    val reactions: List<WsReactionPayload> = emptyList(),
    val clientId: String? = null,
    val isMine: Boolean = false,
    val isEdited: Boolean = false,
    val createdAt: String,
    val moderationVerdict: String? = null,
    val moderationCategories: List<String> = emptyList(),
)

@Serializable
data class WsReplyPayload(
    val messageId: String,
    val senderName: String? = null,
    val body: String? = null,
)

@Serializable
data class WsReactionPayload(
    val emoji: String,
    val count: Int,
    val reactedByMe: Boolean,
    val userIds: List<Int> = emptyList(),
    val senderNames: List<String> = emptyList(),
)

@Serializable
data class WsConversationPayload(
    val id: String,
    val kind: String,
    val title: String? = null,
    val isDirect: Boolean = false,
    val directUserId: String? = null,
    val directUserPid: String? = null,
    val directUserName: String? = null,
    val directUserAvatar: String? = null,
    val directUserStatus: String? = null,
    val unreadCount: Long = 0,
    val latestMessage: String? = null,
    val latestTimestamp: String? = null,
    val latestMessageIsMine: Boolean = false,
    val latestSenderName: String? = null,
    val isBlocked: Boolean = false,
)
