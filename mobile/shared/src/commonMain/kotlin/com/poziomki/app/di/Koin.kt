package com.poziomki.app.di

import org.koin.core.context.startKoin
import org.koin.core.module.Module

fun startAppKoin(appModules: List<Module> = emptyList()) {
    startKoin {
        modules(sharedModule + platformModule() + appModules)
    }
}
