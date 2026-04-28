@file:Suppress("DEPRECATION")

package com.poziomki.app.session

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class AndroidSecureSessionTokenStore(
    context: Context,
) : SessionTokenStore {
    private val sharedPreferences =
        EncryptedSharedPreferences.create(
            context,
            FILE_NAME,
            MasterKey
                .Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build(),
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )

    override suspend fun getToken(): String? =
        withContext(Dispatchers.IO) {
            sharedPreferences.getString(KEY_TOKEN, null)
        }

    // Use commit() (synchronous) rather than apply() so the caller's
    // suspend boundary is the disk-flush boundary. With apply(), the
    // SessionManager.saveSession sequence "saveToken → dataStore.edit"
    // would let the DataStore commit beat the token to disk; a crash
    // between the two leaves USER_ID set with no token, and the app
    // boots into a logged-in-no-token state with no recovery path.
    override suspend fun saveToken(token: String) {
        withContext(Dispatchers.IO) {
            sharedPreferences
                .edit()
                .putString(KEY_TOKEN, token)
                .commit()
        }
    }

    override suspend fun clearToken() {
        withContext(Dispatchers.IO) {
            sharedPreferences
                .edit()
                .remove(KEY_TOKEN)
                .commit()
        }
    }

    private companion object {
        const val FILE_NAME = "poziomki_secure_session"
        const val KEY_TOKEN = "session_token"
    }
}
