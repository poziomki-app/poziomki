@file:Suppress("DEPRECATION")

package com.poziomki.app.session

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

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

    override suspend fun getToken(): String? = sharedPreferences.getString(KEY_TOKEN, null)

    override suspend fun saveToken(token: String) {
        sharedPreferences
            .edit()
            .putString(KEY_TOKEN, token)
            .apply()
    }

    override suspend fun clearToken() {
        sharedPreferences
            .edit()
            .remove(KEY_TOKEN)
            .apply()
    }

    private companion object {
        const val FILE_NAME = "poziomki_secure_session"
        const val KEY_TOKEN = "session_token"
    }
}
