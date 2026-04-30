package com.poziomki.app.chat.push

import com.poziomki.app.network.ApiService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

/**
 * Bridge invoked from Swift `AppDelegate` once iOS hands us an APNs device
 * token (in `application(_:didRegisterForRemoteNotificationsWithDeviceToken:)`).
 *
 * Swift hex-encodes the `Data` token and calls [registerApnsToken] on the
 * shared instance. We resolve the same `deviceId` Android uses, then POST to
 * `/api/v1/chat/push/register` with `platform=ios` so the backend knows to
 * dispatch APNs (not ntfy) for this user.
 *
 * Held as a Koin singleton so callers don't have to build dependencies
 * by hand from Swift.
 */
class IosPushBridge(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    fun registerApnsToken(hexToken: String) {
        if (hexToken.isBlank()) return
        scope.launch {
            // Only register once we have a session — pre-login tokens would
            // 401 anyway. The Swift side may receive the APNs token before
            // the user logs in; in that case we queue it implicitly by
            // doing nothing here and rely on iOS re-issuing the token on
            // next launch (UNUserNotificationCenter caches it).
            if (!sessionManager.isLoggedIn.first()) return@launch
            val deviceId = sessionManager.getOrCreateDeviceId()
            apiService.registerChatPushIos(deviceId = deviceId, apnsToken = hexToken)
        }
    }
}
