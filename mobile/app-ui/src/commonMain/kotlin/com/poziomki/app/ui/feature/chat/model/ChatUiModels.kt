package com.poziomki.app.ui.feature.chat.model

import com.poziomki.app.chat.api.TimelineItem
import com.poziomki.app.chat.api.TimelineMode

sealed interface ComposerMode {
    data object NewMessage : ComposerMode

    data class Reply(
        val eventId: String,
        val senderDisplayName: String?,
        val bodyPreview: String,
    ) : ComposerMode

    data class Edit(
        val eventOrTransactionId: String,
        val originalBody: String,
    ) : ComposerMode
}

data class ChatUiState(
    val roomId: String = "",
    val roomDisplayName: String = "",
    val roomAvatarUrl: String? = null,
    val isDirectRoom: Boolean = false,
    val avatarOverrides: Map<String, String> = emptyMap(),
    val timelineMode: TimelineMode = TimelineMode.Live,
    val timelineItems: List<TimelineItem> = emptyList(),
    val isPaginatingBackwards: Boolean = false,
    val hasMoreBackwards: Boolean = true,
    val isAwayFromLatest: Boolean = false,
    val unreadBelowCount: Int = 0,
    val typingUserIds: List<String> = emptyList(),
    val typingDisplayNames: List<String> = emptyList(),
    val typingAvatarUrls: List<String?> = emptyList(),
    val messageDraft: String = "",
    val composerMode: ComposerMode = ComposerMode.NewMessage,
    val isLoading: Boolean = false,
    val error: String? = null,
)

data class NewChatUiState(
    val profiles: List<com.poziomki.app.network.MatchProfile> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)
