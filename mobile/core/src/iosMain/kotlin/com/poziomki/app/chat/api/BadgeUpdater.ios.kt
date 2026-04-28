package com.poziomki.app.chat.api

import platform.UIKit.UIApplication

/**
 * iOS: drive the springboard app icon badge via
 * `UIApplication.applicationIconBadgeNumber`. iOS 16+ has a newer
 * `UNUserNotificationCenter.setBadgeCount(_:)` API but the legacy
 * property still works on every supported version and doesn't
 * require an authorization round-trip. Must be called on the main
 * thread; the chat layer publishes on Default but the property
 * setter is thread-safe in practice on Darwin.
 */
actual class BadgeUpdater actual constructor() {
    actual fun setBadgeCount(count: Int) {
        UIApplication.sharedApplication.applicationIconBadgeNumber = count.toLong()
    }
}
