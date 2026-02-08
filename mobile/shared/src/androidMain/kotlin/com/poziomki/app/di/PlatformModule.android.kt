package com.poziomki.app.di

import android.content.Context
import com.poziomki.app.session.createDataStore
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.okhttp.OkHttp
import org.koin.core.module.Module
import org.koin.dsl.module

actual fun platformModule(): Module =
    module {
        single<HttpClientEngine> {
            OkHttp.create {
                addInterceptor { chain ->
                    val request =
                        chain
                            .request()
                            .newBuilder()
                            .header("Origin", "http://localhost:3000")
                            .build()
                    chain.proceed(request)
                }
            }
        }
        single { createDataStore(get<Context>()) }
    }
