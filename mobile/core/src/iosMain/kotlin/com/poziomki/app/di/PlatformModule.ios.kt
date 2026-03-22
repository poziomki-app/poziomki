package com.poziomki.app.di

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.native.NativeSqliteDriver
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.chat.api.NoopChatClient
import com.poziomki.app.connectivity.ConnectivityMonitor
import com.poziomki.app.connectivity.IosConnectivityMonitor
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.location.LocationProvider
import com.poziomki.app.session.IosSecureSessionTokenStore
import com.poziomki.app.session.SessionTokenStore
import com.poziomki.app.session.createDataStoreIos
import com.poziomki.app.storage.FileSaver
import com.poziomki.app.storage.IosFileSaver
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.darwin.Darwin
import org.koin.core.module.Module
import org.koin.dsl.module

actual fun platformModule(): Module =
    module {
        single<HttpClientEngine> { Darwin.create() }
        single { createDataStoreIos() }
        single<SessionTokenStore> { IosSecureSessionTokenStore() }
        single<ChatClient> { NoopChatClient() }
        single<SqlDriver> {
            NativeSqliteDriver(PoziomkiDatabase.Schema, "poziomki.db")
        }
        single<ConnectivityMonitor> { IosConnectivityMonitor() }
        single { LocationProvider() }
        single<FileSaver> { IosFileSaver() }
    }
