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
    }

    val screenshotsAllowed: Flow<Boolean> =
        dataStore.data.map { prefs -> prefs[SCREENSHOTS_ALLOWED] ?: true }

    suspend fun setScreenshotsAllowed(allowed: Boolean) {
        dataStore.edit { it[SCREENSHOTS_ALLOWED] = allowed }
    }
}
