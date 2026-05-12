package com.poziomki.app.chat.push

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.poziomki.app.chat.ActiveChat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

/**
 * Receives data-only FCM wake-ups. The server never sends sender names,
 * message bodies, or any other PII in the payload — only the opaque
 * conversation UUID. We show a generic local notification and rely on
 * the user opening the app to see real content over the authenticated
 * API. The user-facing experience is identical to the previous ntfy
 * path; only the transport changed.
 */
class PoziomkiFirebaseMessagingService :
    FirebaseMessagingService(),
    KoinComponent {
    private val notificationHelper: NotificationHelper by inject()
    private val pushManager: PushManager by inject()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onNewToken(token: String) {
        scope.launch { pushManager.onTokenRefreshed(token) }
    }

    override fun onMessageReceived(message: RemoteMessage) {
        val data = message.data
        val roomId = data["conversation_id"]?.takeIf { it.isNotBlank() }
        if (roomId != null && roomId == ActiveChat.roomId) return
        notificationHelper.showMessageNotification(
            sender = null,
            roomId = roomId,
            body = null,
            avatarUrl = null,
            timestampMs = message.sentTime.takeIf { it > 0 },
        )
    }
}
