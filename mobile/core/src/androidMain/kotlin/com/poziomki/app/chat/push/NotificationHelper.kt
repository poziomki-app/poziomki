package com.poziomki.app.chat.push

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

class NotificationHelper(
    private val context: Context,
) {
    private val notificationManager =
        context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    private val notificationIdCounter = AtomicInteger(1000)

    fun createChannels() {
        val messagesChannel =
            NotificationChannel(
                CHANNEL_MESSAGES,
                "Wiadomości",
                NotificationManager.IMPORTANCE_HIGH,
            ).apply {
                description = "Powiadomienia o nowych wiadomościach"
            }
        val serviceChannel =
            NotificationChannel(
                CHANNEL_SERVICE,
                "Usługa push",
                NotificationManager.IMPORTANCE_MIN,
            ).apply {
                description = "Połączenie w tle dla powiadomień push"
            }
        notificationManager.createNotificationChannel(messagesChannel)
        notificationManager.createNotificationChannel(serviceChannel)
    }

    fun buildServiceNotification(): Notification =
        Notification
            .Builder(context, CHANNEL_SERVICE)
            .setContentTitle("Poziomki")
            .setContentText("Connected")
            .setSmallIcon(android.R.drawable.sym_action_chat)
            .setOngoing(true)
            .build()

    fun showMessageNotification(
        sender: String?,
        roomId: String?,
        body: String? = null,
        avatarUrl: String? = null,
        timestampMs: Long? = null,
    ) {
        val title = sender ?: "Poziomki"
        val text = body ?: "Nowa wiadomość"
        val groupKey = "poz_messages_${roomId ?: "unknown"}"
        val notificationTime = timestampMs ?: System.currentTimeMillis()
        val sortKey = notificationTime.toString().padStart(20, '0')

        val builder =
            Notification
                .Builder(context, CHANNEL_MESSAGES)
                .setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.sym_action_chat)
                .setAutoCancel(true)
                .setGroup(groupKey)
                .setWhen(notificationTime)
                .setShowWhen(true)
                .setSortKey(sortKey)

        roomId
            ?.takeIf { it.isNotBlank() }
            ?.let { targetRoomId ->
                buildChatPendingIntent(targetRoomId)?.let(builder::setContentIntent)
            }

        if (avatarUrl != null) {
            runCatching {
                val request = Request.Builder().url(avatarUrl).build()
                avatarClient.newCall(request).execute().use { response ->
                    if (response.isSuccessful) {
                        val bytes = response.body.byteStream().readNBytes(MAX_AVATAR_BYTES)
                        val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
                        if (bitmap != null) builder.setLargeIcon(bitmap)
                    }
                }
            }
        }

        notificationManager.notify(notificationIdCounter.getAndIncrement(), builder.build())

        // Always post group summary so all notifications stack
        val key = roomId ?: "unknown"
        val summaryId = GROUP_SUMMARY_BASE + key.hashCode().and(0x7FFFFFFF)
        val summary =
            Notification
                .Builder(context, CHANNEL_MESSAGES)
                .setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.sym_action_chat)
                .setGroup(groupKey)
                .setGroupSummary(true)
                .setGroupAlertBehavior(Notification.GROUP_ALERT_CHILDREN)
                .setAutoCancel(true)
                .setWhen(notificationTime)
                .setShowWhen(true)
                .build()
        notificationManager.notify(summaryId, summary)
    }

    /**
     * Builds an EXPLICIT PendingIntent that opens the launcher activity.
     *
     * Returns null if — for any reason — the package manager cannot resolve our own
     * launch intent. An implicit fallback would be a CWE-927 vulnerability (a malicious
     * app could hijack the wrapped Intent), so we drop the content intent instead.
     */
    private fun buildChatPendingIntent(roomId: String): PendingIntent? {
        val intent =
            context.packageManager.getLaunchIntentForPackage(context.packageName)
                ?: return null
        intent.flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        intent.putExtra(NotificationChatTarget.EXTRA_OPEN_CHAT_ROOM_ID, roomId)
        return PendingIntent.getActivity(
            context,
            roomId.hashCode(),
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    companion object {
        private const val GROUP_SUMMARY_BASE = 500
        private const val MAX_AVATAR_BYTES = 512 * 1024
        const val CHANNEL_MESSAGES = "poz_messages"
        const val CHANNEL_SERVICE = "poz_push_service"
        const val SERVICE_NOTIFICATION_ID = 900

        private val avatarClient =
            OkHttpClient
                .Builder()
                .connectTimeout(5, TimeUnit.SECONDS)
                .readTimeout(5, TimeUnit.SECONDS)
                .build()
    }
}
