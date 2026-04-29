package com.poziomki.app.di

import org.koin.core.context.startKoin
import org.koin.core.module.Module

fun startAppKoin(
    appModules: List<Module> = emptyList(),
    properties: Map<String, Any> = emptyMap(),
) {
    startKoin {
        if (properties.isNotEmpty()) properties(properties)
        modules(sharedModule + platformModule() + appModules)
    }
}
