package com.poziomki.app.session

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import com.poziomki.app.cache.ImageCacheCleaner
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

data class SessionBootstrapState(
    val isLoggedIn: Boolean,
    val hasProfile: Boolean,
)

@Suppress("TooManyFunctions")
class SessionManager(
    private val dataStore: DataStore<Preferences>,
    private val tokenStore: SessionTokenStore,
    private val imageCacheCleaner: ImageCacheCleaner,
) {
    private companion object {
        val USER_ID = stringPreferencesKey("user_id")
        val USER_EMAIL = stringPreferencesKey("user_email")
        val USER_NAME = stringPreferencesKey("user_name")
        val PROFILE_ID = stringPreferencesKey("profile_id")
        val ONBOARDING_DRAFT = stringPreferencesKey("onboarding_draft")
        val DEVICE_ID = stringPreferencesKey("device_id")
        val LAST_SEEN_VERSION_CODE = intPreferencesKey("last_seen_version_code")

        // Mirrors keys in AppPreferences (same DataStore). Preserved across
        // logouts so onboarding-style flags don't reappear when a user signs
        // back in on the same device.
        val WELCOME_SEEN = booleanPreferencesKey("welcome_seen_v1")
        val FEEDBACK_BANNER_DISMISSED = booleanPreferencesKey("feedback_banner_dismissed_v1")
        val SCREENSHOTS_ALLOWED = booleanPreferencesKey("screenshots_allowed")
    }

    suspend fun getLastSeenVersionCode(): Int? = dataStore.data.first()[LAST_SEEN_VERSION_CODE]

    suspend fun setLastSeenVersionCode(versionCode: Int) {
        dataStore.edit { prefs ->
            prefs[LAST_SEEN_VERSION_CODE] = versionCode
        }
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
        // Order matters: token first (durably flushed by tokenStore),
        // then DataStore. USER_ID is the gate that `isLoggedIn` reads,
        // so committing it last guarantees we never observe
        // logged-in-without-token. If the second write fails, USER_ID
        // stays unset, the app shows the login screen, and the next
        // sign-in overwrites the orphaned token cleanly.
        tokenStore.saveToken(token)
        dataStore.edit { prefs ->
            prefs[USER_ID] = userId
            prefs[USER_EMAIL] = email
            prefs[USER_NAME] = name
        }
    }

    val email: Flow<String?> =
        dataStore.data.map { prefs ->
            prefs[USER_EMAIL]
        }

    suspend fun updateEmail(email: String) {
        dataStore.edit { prefs ->
            prefs[USER_EMAIL] = email
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
        val prefs =
            dataStore.edit { mutablePrefs ->
                if (mutablePrefs[DEVICE_ID] == null) {
                    mutablePrefs[DEVICE_ID] = "android_${kotlin.uuid.Uuid.random()}"
                }
            }
        return prefs[DEVICE_ID]!!
    }

    suspend fun clearSession() {
        // Order matters (mirrors saveSession): clear USER_ID first so
        // `isLoggedIn` immediately observes false, *then* drop the
        // token. If we crash between the two, we'd have an orphaned
        // token but no USER_ID — the app shows the login screen, the
        // next sign-in overwrites the token, and there's no stuck
        // logged-in-without-token state.
        // Preserve device_id and last_seen_version_code across logouts:
        // the former so push-notification routing keeps working, the
        // latter so AppUpdateMigrator still triggers a cache wipe on
        // a post-logout app upgrade (without it, the migrator would
        // see previous=null and treat the upgrade as a first install).
        val snapshot = dataStore.data.first()
        val deviceId = snapshot[DEVICE_ID]
        val lastSeenVersion = snapshot[LAST_SEEN_VERSION_CODE]
        val welcomeSeen = snapshot[WELCOME_SEEN]
        val feedbackBannerDismissed = snapshot[FEEDBACK_BANNER_DISMISSED]
        val screenshotsAllowed = snapshot[SCREENSHOTS_ALLOWED]
        dataStore.edit {
            it.clear()
            if (deviceId != null) it[DEVICE_ID] = deviceId
            if (lastSeenVersion != null) it[LAST_SEEN_VERSION_CODE] = lastSeenVersion
            if (welcomeSeen != null) it[WELCOME_SEEN] = welcomeSeen
            if (feedbackBannerDismissed != null) it[FEEDBACK_BANNER_DISMISSED] = feedbackBannerDismissed
            if (screenshotsAllowed != null) it[SCREENSHOTS_ALLOWED] = screenshotsAllowed
        }
        tokenStore.clearToken()
        runCatching { imageCacheCleaner.clear() }
    }
}
