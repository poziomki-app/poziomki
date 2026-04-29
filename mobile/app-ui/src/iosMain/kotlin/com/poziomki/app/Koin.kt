package com.poziomki.app

import com.poziomki.app.di.appModule
import com.poziomki.app.di.startAppKoin
import com.poziomki.app.ui.cache.ImageCacheCleaner
import com.poziomki.app.ui.cache.NoopImageCacheCleaner
import org.koin.dsl.module

fun initKoin() {
    startAppKoin(
        listOf(
            appModule,
            module {
                single<ImageCacheCleaner> { NoopImageCacheCleaner() }
            },
        ),
    )
}
