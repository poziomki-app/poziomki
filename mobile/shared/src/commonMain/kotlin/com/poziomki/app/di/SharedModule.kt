package com.poziomki.app.di

import com.poziomki.app.api.ApiClient
import com.poziomki.app.api.ApiService
import com.poziomki.app.chat.draft.InMemoryRoomComposerDraftStore
import com.poziomki.app.chat.draft.RoomComposerDraftStore
import com.poziomki.app.data.CacheManager
import com.poziomki.app.data.repository.DegreeRepository
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.ProfileRepository
import com.poziomki.app.data.repository.SettingsRepository
import com.poziomki.app.data.repository.TagRepository
import com.poziomki.app.data.sync.PendingOperationsManager
import com.poziomki.app.data.sync.SyncEngine
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import org.koin.core.module.Module
import org.koin.dsl.module

val sharedModule =
    module {
        single { SessionManager(get(), get()) }
        single<RoomComposerDraftStore> { InMemoryRoomComposerDraftStore() }
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
        single { PoziomkiDatabase(get()) }
        single { CacheManager(get()) }
        single { PendingOperationsManager(get()) }
        single { EventRepository(get(), get(), get(), get(), get()) }
        single { ProfileRepository(get(), get(), get(), get()) }
        single { TagRepository(get(), get()) }
        single { DegreeRepository(get(), get()) }
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
