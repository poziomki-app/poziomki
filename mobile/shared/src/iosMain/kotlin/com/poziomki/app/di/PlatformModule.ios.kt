package com.poziomki.app.di

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.native.NativeSqliteDriver
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.NoopMatrixClient
import com.poziomki.app.data.connectivity.ConnectivityMonitor
import com.poziomki.app.data.connectivity.IosConnectivityMonitor
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.session.IosSecureSessionTokenStore
import com.poziomki.app.session.SessionTokenStore
import com.poziomki.app.session.createDataStoreIos
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.darwin.Darwin
import org.koin.core.module.Module
import org.koin.dsl.module

actual fun platformModule(): Module =
    module {
        single<HttpClientEngine> { Darwin.create() }
        single { createDataStoreIos() }
        single<SessionTokenStore> { IosSecureSessionTokenStore() }
        single<MatrixClient> { NoopMatrixClient() }
        single<SqlDriver> {
            NativeSqliteDriver(PoziomkiDatabase.Schema, "poziomki.db")
        }
        single<ConnectivityMonitor> { IosConnectivityMonitor() }
        single { LocationProvider() }
    }
