package com.poziomki.app.data.repository

import app.cash.sqldelight.coroutines.asFlow
import app.cash.sqldelight.coroutines.mapToOneOrNull
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.db.User_settings
import com.poziomki.app.network.UpdateSettingsRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlin.time.Clock

class SettingsRepository(
    private val db: PoziomkiDatabase,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
) {
    private val json = Json { explicitNulls = false }

    private fun defaultSettings(userId: String): User_settings =
        User_settings(
            user_id = userId,
            theme = "system",
            language = "system",
            notifications_enabled = 1L,
            privacy_show_program = 1L,
            privacy_discoverable = 1L,
            cached_at = Clock.System.now().toEpochMilliseconds(),
            is_dirty = 0L,
        )

    private fun upsertSettings(settings: User_settings) {
        db.userSettingsQueries.upsert(
            user_id = settings.user_id,
            theme = settings.theme,
            language = settings.language,
            notifications_enabled = settings.notifications_enabled,
            privacy_show_program = settings.privacy_show_program,
            privacy_discoverable = settings.privacy_discoverable,
            cached_at = settings.cached_at,
            is_dirty = settings.is_dirty,
        )
    }

    private fun getOrCreateSettings(userId: String): User_settings =
        db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull()
            ?: defaultSettings(userId).also(::upsertSettings)

    fun observeSettings(userId: String): Flow<User_settings?> =
        db.userSettingsQueries
            .selectByUserId(userId)
            .asFlow()
            .mapToOneOrNull(Dispatchers.IO)

    suspend fun ensureDefaults(userId: String) {
        withContext(Dispatchers.IO) {
            getOrCreateSettings(userId)
        }
    }

    suspend fun updateTheme(
        userId: String,
        theme: String,
    ) {
        withContext(Dispatchers.IO) {
            val current = getOrCreateSettings(userId)
            upsertSettings(
                current.copy(
                    theme = theme,
                    is_dirty = 1L,
                ),
            )
            pendingOps.enqueue("update_settings", userId, json.encodeToString(UpdateSettingsRequest(theme = theme)))
        }
    }

    suspend fun updateLanguage(
        userId: String,
        language: String,
    ) {
        withContext(Dispatchers.IO) {
            val current = getOrCreateSettings(userId)
            upsertSettings(
                current.copy(
                    language = language,
                    is_dirty = 1L,
                ),
            )
            pendingOps.enqueue("update_settings", userId, json.encodeToString(UpdateSettingsRequest(language = language)))
        }
    }

    suspend fun updateNotifications(
        userId: String,
        enabled: Boolean,
    ) {
        withContext(Dispatchers.IO) {
            val current = getOrCreateSettings(userId)
            upsertSettings(
                current.copy(
                    notifications_enabled = if (enabled) 1L else 0L,
                    is_dirty = 1L,
                ),
            )
            pendingOps.enqueue("update_settings", userId, json.encodeToString(UpdateSettingsRequest(notificationsEnabled = enabled)))
        }
    }

    suspend fun updatePrivacy(
        userId: String,
        showProgram: Boolean? = null,
        discoverable: Boolean? = null,
    ) {
        withContext(Dispatchers.IO) {
            val current = getOrCreateSettings(userId)
            upsertSettings(
                current.copy(
                    privacy_show_program = showProgram?.let { if (it) 1L else 0L } ?: current.privacy_show_program,
                    privacy_discoverable = discoverable?.let { if (it) 1L else 0L } ?: current.privacy_discoverable,
                    is_dirty = 1L,
                ),
            )
            pendingOps.enqueue(
                "update_settings",
                userId,
                json.encodeToString(
                    UpdateSettingsRequest(
                        privacyShowProgram = showProgram,
                        privacyDiscoverable = discoverable,
                    ),
                ),
            )
        }
    }
}
