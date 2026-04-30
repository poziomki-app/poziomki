package com.poziomki.app

import com.poziomki.app.cache.ImageCacheCleaner
import com.poziomki.app.cache.NoopImageCacheCleaner
import com.poziomki.app.chat.push.IosPushBridge
import com.poziomki.app.di.appModule
import com.poziomki.app.di.startAppKoin
import org.koin.dsl.module
import org.koin.mp.KoinPlatform

fun initKoin(versionCode: Int) {
    startAppKoin(
        appModules =
            listOf(
                appModule,
                module {
                    single<ImageCacheCleaner> { NoopImageCacheCleaner() }
                },
            ),
        properties = mapOf("APP_VERSION_CODE" to versionCode),
    )
}

/**
 * Bridge entry point invoked from Swift `AppDelegate` once iOS hands us
 * an APNs device token. Resolves the [IosPushBridge] Koin singleton and
 * forwards the hex-encoded token. Safe to call before login — the bridge
 * no-ops if the user isn't authenticated yet.
 */
fun registerApnsToken(hexToken: String) {
    KoinPlatform.getKoin().get<IosPushBridge>().registerApnsToken(hexToken)
}
