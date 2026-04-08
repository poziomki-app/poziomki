package com.poziomki.app.chat.push

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import java.net.URL
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
                "Messages",
                NotificationManager.IMPORTANCE_HIGH,
            ).apply {
                description = "New message notifications"
            }
        val serviceChannel =
            NotificationChannel(
                CHANNEL_SERVICE,
                "Push Service",
                NotificationManager.IMPORTANCE_MIN,
            ).apply {
                description = "Background connection for push notifications"
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
        val title = sender ?: "New message"
        val text = body ?: "You have a new message"
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
                builder.setContentIntent(buildChatPendingIntent(targetRoomId))
            }

        if (avatarUrl != null) {
            runCatching {
                val bitmap = URL(avatarUrl).openStream().use { BitmapFactory.decodeStream(it) }
                if (bitmap != null) builder.setLargeIcon(bitmap)
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

    private fun buildChatPendingIntent(roomId: String): PendingIntent {
        val intent =
            context.packageManager.getLaunchIntentForPackage(context.packageName)?.apply {
                flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
                putExtra(NotificationChatTarget.EXTRA_OPEN_CHAT_ROOM_ID, roomId)
            } ?: Intent().apply {
                putExtra(NotificationChatTarget.EXTRA_OPEN_CHAT_ROOM_ID, roomId)
            }
        return PendingIntent.getActivity(
            context,
            roomId.hashCode(),
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    companion object {
        private const val GROUP_SUMMARY_BASE = 500
        const val CHANNEL_MESSAGES = "poz_messages"
        const val CHANNEL_SERVICE = "poz_push_service"
        const val SERVICE_NOTIFICATION_ID = 900
    }
}
