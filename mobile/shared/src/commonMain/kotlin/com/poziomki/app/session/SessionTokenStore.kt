package com.poziomki.app.session

interface SessionTokenStore {
    suspend fun getToken(): String?

    suspend fun saveToken(token: String)

    suspend fun clearToken()
}
