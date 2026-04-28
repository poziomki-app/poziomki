package com.poziomki.app.chat.api

/**
 * Platform-specific launcher/notification badge updater. Called by
 * the chat layer whenever [ChatClient.totalUnread] changes. The
 * common layer doesn't try to hide badge differences — Android's
 * shortcut badge and iOS's notification badge have different
 * semantics — so this is just a thin actual-per-platform.
 *
 * Implementations should be cheap and idempotent: invoked on every
 * unread tick.
 */
expect class BadgeUpdater() {
    fun setBadgeCount(count: Int)
}
