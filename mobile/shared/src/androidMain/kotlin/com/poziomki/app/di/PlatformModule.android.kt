package com.poziomki.app.di

import android.content.Context
import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.android.AndroidSqliteDriver
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.impl.RustMatrixClient
import com.poziomki.app.chat.push.NotificationHelper
import com.poziomki.app.chat.push.PushManager
import com.poziomki.app.data.connectivity.AndroidConnectivityMonitor
import com.poziomki.app.data.connectivity.ConnectivityMonitor
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.session.AndroidSecureSessionTokenStore
import com.poziomki.app.session.SessionTokenStore
import com.poziomki.app.session.createDataStore
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.okhttp.OkHttp
import org.koin.core.module.Module
import org.koin.dsl.module

actual fun platformModule(): Module =
    module {
        single<HttpClientEngine> {
            val apiUrl = getProperty("API_BASE_URL", "http://localhost:5150")
            OkHttp.create {
                addInterceptor { chain ->
                    val request =
                        chain
                            .request()
                            .newBuilder()
                            .header("Origin", apiUrl)
                            .build()
                    chain.proceed(request)
                }
            }
        }
        single { createDataStore(get<Context>()) }
        single<SessionTokenStore> { AndroidSecureSessionTokenStore(get<Context>()) }
        single<MatrixClient> { RustMatrixClient(get(), get(), get()) }
        single<SqlDriver> {
            AndroidSqliteDriver(PoziomkiDatabase.Schema, get<Context>(), "poziomki.db")
        }
        single<ConnectivityMonitor> { AndroidConnectivityMonitor(get<Context>()) }
        single { LocationProvider(get<Context>()) }
        single { NotificationHelper(get<Context>()) }
        single { PushManager(get(), get(), get<Context>()) }
    }
