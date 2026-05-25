package com.poziomki.app

import com.poziomki.app.cache.ImageCacheCleaner
import com.poziomki.app.cache.NoopImageCacheCleaner
import com.poziomki.app.di.appModule
import com.poziomki.app.di.startAppKoin
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.runBlocking
import org.koin.dsl.module
import org.koin.mp.KoinPlatform

fun initKoin(
    versionCode: Int,
    apiBaseUrl: String,
) {
    startAppKoin(
        appModules =
            listOf(
                appModule,
                module {
                    single<ImageCacheCleaner> { NoopImageCacheCleaner() }
                },
            ),
        properties =
            mapOf(
                "APP_VERSION_CODE" to versionCode,
                "API_BASE_URL" to apiBaseUrl,
            ),
    )
}

// E2E test helper. Bypasses the login flow by writing a pre-fetched session
// directly into SessionManager. Invoked from iOSApp.swift only when the
// POZIOMKI_REVIEW_TOKEN env var is set on the simulator launch — never on
// real device builds. Without those env vars, this is a no-op.
fun injectReviewSession(
    token: String,
    userId: String,
    email: String,
    name: String,
) {
    val sessionManager = KoinPlatform.getKoin().get<SessionManager>()
    runBlocking {
        sessionManager.saveSession(token = token, userId = userId, email = email, name = name)
    }
}
