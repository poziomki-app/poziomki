package com.poziomki.app.session

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

data class SessionBootstrapState(
    val isLoggedIn: Boolean,
    val hasProfile: Boolean,
)

class SessionManager(
    private val dataStore: DataStore<Preferences>,
    private val tokenStore: SessionTokenStore,
) {
    private companion object {
        val USER_ID = stringPreferencesKey("user_id")
        val USER_EMAIL = stringPreferencesKey("user_email")
        val USER_NAME = stringPreferencesKey("user_name")
        val PROFILE_ID = stringPreferencesKey("profile_id")
        val ONBOARDING_DRAFT = stringPreferencesKey("onboarding_draft")
        val DEVICE_ID = stringPreferencesKey("device_id")
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

    suspend fun getProfileId(): String? = dataStore.data.first()[PROFILE_ID]

    suspend fun getBootstrapState(): SessionBootstrapState {
        val prefs = dataStore.data.first()
        return SessionBootstrapState(
            isLoggedIn = prefs[USER_ID] != null,
            hasProfile = prefs[PROFILE_ID] != null,
        )
    }

    suspend fun saveOnboardingDraft(draftJson: String?) {
        dataStore.edit { prefs ->
            if (draftJson.isNullOrBlank()) {
                prefs.remove(ONBOARDING_DRAFT)
            } else {
                prefs[ONBOARDING_DRAFT] = draftJson
            }
        }
    }

    suspend fun getOnboardingDraft(): String? = dataStore.data.first()[ONBOARDING_DRAFT]

    @OptIn(kotlin.uuid.ExperimentalUuidApi::class)
    suspend fun getOrCreateDeviceId(): String {
        val prefs = dataStore.edit { mutablePrefs ->
            if (mutablePrefs[DEVICE_ID] == null) {
                mutablePrefs[DEVICE_ID] = "android_${kotlin.uuid.Uuid.random()}"
            }
        }
        return prefs[DEVICE_ID]!!
    }

    suspend fun clearSession() {
        tokenStore.clearToken()
        // Preserve device_id across logouts
        val deviceId = dataStore.data.first()[DEVICE_ID]
        dataStore.edit {
            it.clear()
            if (deviceId != null) it[DEVICE_ID] = deviceId
        }
    }
}
