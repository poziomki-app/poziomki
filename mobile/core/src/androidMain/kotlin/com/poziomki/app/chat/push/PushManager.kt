package com.poziomki.app.chat.push

import com.google.firebase.messaging.FirebaseMessaging
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.api.ChatClientState
import com.poziomki.app.network.ApiService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Tracks the FCM token and keeps the backend's `push_subscriptions` row
 * in sync with whichever device the user is currently signed in on.
 * On sign-in we fetch the current FCM token and register it; on sign-out
 * we unregister so a logged-out device stops receiving wake-ups.
 */
class PushManager(
    private val chatClient: ChatClient,
    @Suppress("UnusedPrivateProperty") private val apiService: ApiService,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    @Volatile
    private var registeredDeviceId: String? = null

    fun startObserving() {
        scope.launch {
            chatClient.state.collectLatest { state ->
                when (state) {
                    is ChatClientState.Ready -> {
                        registerCurrentToken(state.deviceId)
                    }

                    is ChatClientState.Idle -> {
                        unregister()
                    }

                    else -> {}
                }
            }
        }
    }

    /** Called by [PoziomkiFirebaseMessagingService.onNewToken] when FCM rotates the token. */
    suspend fun onTokenRefreshed(token: String) {
        if (registeredDeviceId != null) {
            chatClient.registerPusher(token, PLATFORM_ANDROID)
        }
    }

    private suspend fun registerCurrentToken(deviceId: String) {
        val token =
            runCatching { fetchToken() }
                .getOrNull() ?: return
        chatClient.registerPusher(token, PLATFORM_ANDROID)
        registeredDeviceId = deviceId
    }

    private suspend fun unregister() {
        if (registeredDeviceId == null) return
        runCatching { chatClient.unregisterPusher() }
        registeredDeviceId = null
    }

    private suspend fun fetchToken(): String =
        suspendCancellableCoroutine { cont ->
            FirebaseMessaging
                .getInstance()
                .token
                .addOnSuccessListener { t -> cont.resume(t) }
                .addOnFailureListener { e -> cont.resumeWithException(e) }
        }

    companion object {
        private const val PLATFORM_ANDROID = "android"
    }
}
