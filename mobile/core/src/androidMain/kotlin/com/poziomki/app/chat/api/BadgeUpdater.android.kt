package com.poziomki.app.chat.api

/**
 * Android: launcher badge support is OEM-specific (Samsung uses a
 * BadgeProvider, Google's launcher reads `Notification.setNumber`).
 * The most reliable common path is to attach the count to the
 * persistent chat notification, which we don't yet maintain in core.
 *
 * For now this is a no-op stub; the higher-level Android app can
 * supply a real implementation via DI when it owns the active
 * notification handle. Keeping the surface here so the common code
 * doesn't need a `// TODO platform-specific` branch.
 */
actual class BadgeUpdater actual constructor() {
    actual fun setBadgeCount(count: Int) {
        // no-op; replaced by the Android app module when wiring the
        // foreground/notification badge.
    }
}
