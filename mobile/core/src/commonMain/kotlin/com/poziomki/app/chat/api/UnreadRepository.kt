package com.poziomki.app.chat.api

import kotlinx.coroutines.flow.StateFlow

/**
 * Read-only view of the badge-driving unread total. Implementations
 * delegate to [ChatClient.totalUnread] for value, but the repository
 * decouples platform badge integrators (iOS `UNUserNotificationCenter`,
 * Android shortcut/notification badges) from the chat client so the
 * launcher badge can be wired up without dragging the entire chat
 * stack into the platform layer.
 *
 * The total excludes muted conversations — same rule the server
 * applies in `unread_summary_for_user`.
 */
interface UnreadRepository {
    val totalUnread: StateFlow<Int>
}

/** Default repository that simply forwards [ChatClient.totalUnread]. */
class ChatClientUnreadRepository(
    private val chatClient: ChatClient,
) : UnreadRepository {
    override val totalUnread: StateFlow<Int> = chatClient.totalUnread
}
