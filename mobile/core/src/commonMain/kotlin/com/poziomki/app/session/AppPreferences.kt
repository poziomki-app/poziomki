package com.poziomki.app.session

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

class AppPreferences(
    private val dataStore: DataStore<Preferences>,
) {
    private companion object {
        val SCREENSHOTS_ALLOWED = booleanPreferencesKey("screenshots_allowed")
        val WELCOME_SEEN = booleanPreferencesKey("welcome_seen_v1")
        val FEEDBACK_BANNER_DISMISSED = booleanPreferencesKey("feedback_banner_dismissed_v1")
    }

    val screenshotsAllowed: Flow<Boolean> =
        dataStore.data.map { prefs -> prefs[SCREENSHOTS_ALLOWED] ?: true }

    suspend fun setScreenshotsAllowed(allowed: Boolean) {
        dataStore.edit { it[SCREENSHOTS_ALLOWED] = allowed }
    }

    val welcomeSeen: Flow<Boolean> =
        dataStore.data.map { prefs -> prefs[WELCOME_SEEN] ?: false }

    suspend fun setWelcomeSeen(seen: Boolean) {
        dataStore.edit { it[WELCOME_SEEN] = seen }
    }

    val feedbackBannerDismissed: Flow<Boolean> =
        dataStore.data.map { prefs -> prefs[FEEDBACK_BANNER_DISMISSED] ?: false }

    suspend fun setFeedbackBannerDismissed(dismissed: Boolean) {
        dataStore.edit { it[FEEDBACK_BANNER_DISMISSED] = dismissed }
    }
}
