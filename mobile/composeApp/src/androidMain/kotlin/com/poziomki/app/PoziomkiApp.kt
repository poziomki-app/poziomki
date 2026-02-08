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
            modules(sharedModule, platformModule(), appModule)
        }
    }
}
