package com.poziomki.app

import com.poziomki.app.di.appModule
import com.poziomki.app.di.startAppKoin

fun initKoin() {
    startAppKoin(listOf(appModule))
}
