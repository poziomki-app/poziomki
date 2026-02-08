package com.poziomki.app.di

import com.poziomki.app.api.ApiClient
import com.poziomki.app.api.ApiService
import com.poziomki.app.session.SessionManager
import org.koin.core.module.Module
import org.koin.dsl.module

val sharedModule =
    module {
        single { SessionManager(get()) }
        single {
            val sessionManager = get<SessionManager>()
            ApiClient(
                baseUrl = getProperty("API_BASE_URL", "http://localhost:3000"),
                engine = get(),
                tokenProvider = { sessionManager.getToken() },
            )
        }
        single { ApiService(get()) }
    }

expect fun platformModule(): Module
