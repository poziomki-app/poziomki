package com.poziomki.app

import com.poziomki.app.cache.ImageCacheCleaner
import com.poziomki.app.cache.NoopImageCacheCleaner
import com.poziomki.app.di.appModule
import com.poziomki.app.di.startAppKoin
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
