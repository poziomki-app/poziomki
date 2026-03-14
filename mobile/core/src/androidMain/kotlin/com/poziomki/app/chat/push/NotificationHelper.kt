package com.poziomki.app.chat.push

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
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
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setOngoing(true)
            .build()

    fun showMessageNotification(
        sender: String?,
        roomId: String?,
        body: String? = null,
        avatarUrl: String? = null,
    ) {
        val title = sender ?: "New message"
        val text = body ?: "You have a new message"
        val groupKey = "poz_messages_${roomId ?: "unknown"}"

        val builder =
            Notification
                .Builder(context, CHANNEL_MESSAGES)
                .setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.ic_dialog_email)
                .setAutoCancel(true)
                .setGroup(groupKey)

        if (avatarUrl != null) {
            runCatching {
                val bitmap = URL(avatarUrl).openStream().use { BitmapFactory.decodeStream(it) }
                if (bitmap != null) builder.setLargeIcon(bitmap)
            }
        }

        notificationManager.notify(notificationIdCounter.getAndIncrement(), builder.build())

        // Post/update group summary so Android stacks notifications from the same room
        val summaryId = GROUP_SUMMARY_BASE + (roomId?.hashCode() ?: 0)
        val summary =
            Notification
                .Builder(context, CHANNEL_MESSAGES)
                .setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.ic_dialog_email)
                .setGroup(groupKey)
                .setGroupSummary(true)
                .setAutoCancel(true)
                .build()
        notificationManager.notify(summaryId, summary)
    }

    companion object {
        private const val GROUP_SUMMARY_BASE = 500
        const val CHANNEL_MESSAGES = "poz_messages"
        const val CHANNEL_SERVICE = "poz_push_service"
        const val SERVICE_NOTIFICATION_ID = 900
    }
}
