package com.poziomki.app.session

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

class SessionManager(
    private val dataStore: DataStore<Preferences>,
    private val tokenStore: SessionTokenStore,
) {
    private companion object {
        val USER_ID = stringPreferencesKey("user_id")
        val USER_EMAIL = stringPreferencesKey("user_email")
        val USER_NAME = stringPreferencesKey("user_name")
        val PROFILE_ID = stringPreferencesKey("profile_id")
    }

    val isLoggedIn: Flow<Boolean> =
        dataStore.data.map { prefs ->
            prefs[USER_ID] != null
        }

    val userId: Flow<String?> =
        dataStore.data.map { prefs ->
            prefs[USER_ID]
        }

    val sessionToken: Flow<String?> =
        userId.map { tokenStore.getToken() }

    val profileId: Flow<String?> =
        dataStore.data.map { prefs ->
            prefs[PROFILE_ID]
        }

    suspend fun getToken(): String? = tokenStore.getToken()

    suspend fun saveSession(
        token: String,
        userId: String,
        email: String,
        name: String,
    ) {
        tokenStore.saveToken(token)
        dataStore.edit { prefs ->
            prefs[USER_ID] = userId
            prefs[USER_EMAIL] = email
            prefs[USER_NAME] = name
        }
    }

    suspend fun saveProfileId(profileId: String) {
        dataStore.edit { prefs ->
            prefs[PROFILE_ID] = profileId
        }
    }

    suspend fun clearSession() {
        tokenStore.clearToken()
        dataStore.edit { it.clear() }
    }
}
