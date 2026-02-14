package com.poziomki.app

import android.app.Application
import com.poziomki.app.di.appModule
import com.poziomki.app.di.platformModule
import com.poziomki.app.di.sharedModule
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin

class PoziomkiApp : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin {
            androidContext(this@PoziomkiApp)
            properties(
                mapOf(
                    "API_BASE_URL" to BuildConfig.API_BASE_URL,
                    "ENABLE_HTTP_LOGGING" to BuildConfig.DEBUG,
                ),
            )
            modules(sharedModule, platformModule(), appModule)
        }
    }
}
