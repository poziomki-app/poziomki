package com.poziomki.app

import android.app.Application
import android.content.ComponentCallbacks2
import coil3.imageLoader
import com.poziomki.app.cache.ImageCacheCleaner
import com.poziomki.app.chat.push.NotificationHelper
import com.poziomki.app.chat.push.PushManager
import com.poziomki.app.di.appModule
import com.poziomki.app.di.platformModule
import com.poziomki.app.di.sharedModule
import com.poziomki.app.ui.cache.AndroidImageCacheCleaner
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import org.koin.dsl.module
import org.koin.java.KoinJavaComponent.getKoin

class PoziomkiApp : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin {
            androidContext(this@PoziomkiApp)
            properties(
                mapOf(
                    "API_BASE_URL" to BuildConfig.API_BASE_URL,
                    "ENABLE_HTTP_LOGGING" to BuildConfig.DEBUG,
                    "APP_VERSION_CODE" to BuildConfig.VERSION_CODE,
                ),
            )
            modules(
                sharedModule,
                platformModule(),
                appModule,
                module {
                    single<ImageCacheCleaner> { AndroidImageCacheCleaner(get()) }
                },
            )
        }

        getKoin().get<NotificationHelper>().createChannels()
        // Note: PushManager.startObserving and other DataStore/SQL consumers
        // are gated by AppUpdateMigrator.ready inside App.kt — the migrator
        // must finish before anything writes to the caches it might wipe.
        getKoin().get<PushManager>().startObserving()
    }

    @Suppress("DEPRECATION")
    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        if (level >= ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW) {
            imageLoader.memoryCache?.clear()
        }
    }
}
