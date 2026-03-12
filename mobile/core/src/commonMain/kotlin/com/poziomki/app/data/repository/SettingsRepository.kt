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
import kotlinx.datetime.Clock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class SettingsRepository(
    private val db: PoziomkiDatabase,
    private val connectivityMonitor: ConnectivityMonitor,
    private val pendingOps: PendingOperationsManager,
) {
    private val json = Json { explicitNulls = false }

    fun observeSettings(userId: String): Flow<User_settings?> =
        db.userSettingsQueries
            .selectByUserId(userId)
            .asFlow()
            .mapToOneOrNull(Dispatchers.IO)

    suspend fun ensureDefaults(userId: String) {
        withContext(Dispatchers.IO) {
            val existing = db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull()
            if (existing == null) {
                db.userSettingsQueries.upsert(
                    user_id = userId,
                    theme = "system",
                    language = "system",
                    notifications_enabled = 1L,
                    privacy_show_program = 1L,
                    privacy_discoverable = 1L,
                    cached_at = Clock.System.now().toEpochMilliseconds(),
                    is_dirty = 0L,
                )
            }
        }
    }

    suspend fun updateTheme(
        userId: String,
        theme: String,
    ) {
        withContext(Dispatchers.IO) {
            val current = db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull() ?: return@withContext
            db.userSettingsQueries.upsert(
                user_id = userId,
                theme = theme,
                language = current.language,
                notifications_enabled = current.notifications_enabled,
                privacy_show_program = current.privacy_show_program,
                privacy_discoverable = current.privacy_discoverable,
                cached_at = current.cached_at,
                is_dirty = 1L,
            )
            pendingOps.enqueue("update_settings", userId, json.encodeToString(UpdateSettingsRequest(theme = theme)))
        }
    }

    suspend fun updateLanguage(
        userId: String,
        language: String,
    ) {
        withContext(Dispatchers.IO) {
            val current = db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull() ?: return@withContext
            db.userSettingsQueries.upsert(
                user_id = userId,
                theme = current.theme,
                language = language,
                notifications_enabled = current.notifications_enabled,
                privacy_show_program = current.privacy_show_program,
                privacy_discoverable = current.privacy_discoverable,
                cached_at = current.cached_at,
                is_dirty = 1L,
            )
            pendingOps.enqueue("update_settings", userId, json.encodeToString(UpdateSettingsRequest(language = language)))
        }
    }

    suspend fun updateNotifications(
        userId: String,
        enabled: Boolean,
    ) {
        withContext(Dispatchers.IO) {
            val current = db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull() ?: return@withContext
            db.userSettingsQueries.upsert(
                user_id = userId,
                theme = current.theme,
                language = current.language,
                notifications_enabled = if (enabled) 1L else 0L,
                privacy_show_program = current.privacy_show_program,
                privacy_discoverable = current.privacy_discoverable,
                cached_at = current.cached_at,
                is_dirty = 1L,
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
            val current = db.userSettingsQueries.selectByUserId(userId).executeAsOneOrNull() ?: return@withContext
            db.userSettingsQueries.upsert(
                user_id = userId,
                theme = current.theme,
                language = current.language,
                notifications_enabled = current.notifications_enabled,
                privacy_show_program = showProgram?.let { if (it) 1L else 0L } ?: current.privacy_show_program,
                privacy_discoverable = discoverable?.let { if (it) 1L else 0L } ?: current.privacy_discoverable,
                cached_at = current.cached_at,
                is_dirty = 1L,
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
