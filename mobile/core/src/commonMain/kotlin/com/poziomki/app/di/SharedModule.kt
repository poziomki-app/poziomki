package com.poziomki.app.di

import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.chat.cache.SqlDelightRoomTimelineCacheStore
import com.poziomki.app.chat.draft.RoomComposerDraftStore
import com.poziomki.app.chat.draft.SqlDelightRoomComposerDraftStore
import com.poziomki.app.chat.ws.WsConnection
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.ChatRoomRepository
import com.poziomki.app.data.repository.DegreeRepository
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.MatchProfileRepository
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.data.repository.TagRepository
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.data.sync.SyncEngine
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.network.ApiClient
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.GeocodingService
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import org.koin.core.module.Module
import org.koin.dsl.module

val sharedModule =
    module {
        single { SessionManager(get(), get()) }
        single<RoomComposerDraftStore> { SqlDelightRoomComposerDraftStore(get()) }
        single<RoomTimelineCacheStore> { SqlDelightRoomTimelineCacheStore(get()) }
        single {
            val sessionManager = get<SessionManager>()
            ApiClient(
                baseUrl = getProperty("API_BASE_URL", "http://localhost:5150"),
                engine = get(),
                enableHttpLogging = getProperty("ENABLE_HTTP_LOGGING", false),
                tokenProvider = { sessionManager.getToken() },
                onUnauthorized = { sessionManager.clearSession() },
            )
        }
        single { ApiService(get()) }
        single {
            val sessionManager = get<SessionManager>()
            WsConnection(
                baseUrl = getProperty("API_BASE_URL", "http://localhost:5150"),
                tokenProvider = { sessionManager.getToken() },
                engine = get(),
            )
        }
        single { GeocodingService(get()) }
        single { PoziomkiDatabase(get()) }
        single { CacheManager(get()) }
        single { PendingOperationsManager(get()) }
        single { ChatRoomRepository(get()) }
        single { EventRepository(get(), get(), get(), get(), get(), get()) }
        single { ProfileRepository(get(), get(), get(), get()) }
        single { TagRepository(get(), get()) }
        single { DegreeRepository(get(), get()) }
        single { MatchProfileRepository(get(), get()) }
        single { SettingsRepository(get(), get(), get()) }
        single {
            SyncEngine(
                pendingOps = get(),
                api = get(),
                db = get(),
                connectivityMonitor = get(),
                cacheManager = get(),
                scope = CoroutineScope(SupervisorJob() + Dispatchers.Default),
            )
        }
    }

expect fun platformModule(): Module
