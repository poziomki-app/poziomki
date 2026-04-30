package com.poziomki.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.annotation.ExperimentalCoilApi
import coil3.compose.setSingletonImageLoaderFactory
import coil3.disk.DiskCache
import coil3.memory.MemoryCache
import coil3.network.ktor3.KtorNetworkFetcherFactory
import coil3.request.crossfade
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.data.repository.EventRepository
import com.poziomki.app.data.repository.XpRepository
import com.poziomki.app.data.sync.SyncEngine
import com.poziomki.app.session.SessionBootstrapState
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.cache.AppUpdateMigrator
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.navigation.AppNavigation
import com.poziomki.app.ui.navigation.Route
import com.poziomki.app.ui.shared.ImgproxyCacheInterceptor
import com.poziomki.app.ui.shared.coilDiskCachePath
import io.ktor.client.HttpClient
import io.ktor.client.engine.HttpClientEngine
import org.koin.compose.koinInject
import org.koin.mp.KoinPlatform

private const val COIL_MEMORY_CACHE_MAX_SIZE_PERCENT = 0.18
private const val COIL_DISK_CACHE_MAX_SIZE_BYTES = 48L * 1024L * 1024L

@Composable
@Suppress("LongMethod")
fun App() {
    val engine = koinInject<HttpClientEngine>()
    val chatClient = koinInject<ChatClient>()
    val imageHttpClient = remember(engine) { HttpClient(engine) }
    val imageLoaderFactory: (PlatformContext) -> ImageLoader =
        remember(imageHttpClient) { buildImageLoaderFactory(imageHttpClient) }

    setSingletonImageLoaderFactory(imageLoaderFactory)

    val sessionManager = koinInject<SessionManager>()
    val syncEngine = koinInject<SyncEngine>()
    val migrator = koinInject<AppUpdateMigrator>()
    val bootstrapState by
        produceState<SessionBootstrapState?>(initialValue = null, sessionManager) {
            value = sessionManager.getBootstrapState()
        }
    val isLoggedIn by sessionManager.isLoggedIn.collectAsState(initial = bootstrapState?.isLoggedIn ?: false)

    // Block the entire navigation graph until the migration finishes —
    // ViewModels are instantiated on first render of their screens and
    // many of them (ExploreViewModel, EventsViewModel, MessagesViewModel)
    // immediately fetch from the same caches the migrator might wipe.
    // Gating just App.kt's LaunchedEffects isn't enough; we have to gate
    // AppNavigation itself.
    val migrationReady by produceState(initialValue = false, migrator) {
        migrator.ready.await()
        value = true
    }

    // Compute start destination once from initial session state.
    // Not reactive — sign-up saving a session mid-flow must NOT change startDestination.
    val startDestination =
        remember(bootstrapState) {
            val state = bootstrapState
            if (state == null) return@remember null
            when {
                !state.isLoggedIn -> Route.AuthGraph
                !state.hasProfile -> Route.OnboardingGraph
                else -> Route.MainGraph
            }
        }

    // Run the per-version cache migration as the very first thing on the
    // composition coroutine so consumers gated on migrator.ready unblock as
    // soon as it finishes. APP_VERSION_CODE is a Koin property set by each
    // platform entry point (PoziomkiApp on Android, initKoin on iOS).
    LaunchedEffect(Unit) {
        val versionCode = KoinPlatform.getKoin().getProperty("APP_VERSION_CODE", 0)
        runCatching { migrator.runIfVersionChanged(versionCode) }
    }

    // Sync engine starts collecting flows that read from DB; gate it on the
    // migrator so an upgrade-time wipe can't race with active reads.
    LaunchedEffect(Unit) {
        migrator.ready.await()
        syncEngine.start()
    }
    DisposableEffect(Unit) {
        onDispose {
            syncEngine.stop()
            imageHttpClient.close()
        }
    }

    // Warm up chat client + event metadata in background so room previews
    // and chat headers have correct event covers from first paint. Wait for
    // the migration first — chatClient.ensureStarted opens the WS, which
    // immediately starts populating room/timeline caches that the migrator
    // might otherwise wipe out from underneath it.
    val eventRepository = koinInject<EventRepository>()
    LaunchedEffect(startDestination) {
        if (startDestination == Route.MainGraph) {
            migrator.ready.await()
            chatClient.ensureStarted()
            runCatching { eventRepository.refreshMyEvents() }
        }
    }

    // Auto-award daily_login XP when user opens the app already logged in.
    // Backend is idempotent per (profile_id, task_id, day) so re-fires are no-ops.
    val xpRepository = koinInject<XpRepository>()
    LaunchedEffect(isLoggedIn) {
        if (isLoggedIn) {
            migrator.ready.await()
            runCatching { xpRepository.claimTask("daily_login") }
        }
    }

    PoziomkiTheme {
        if (startDestination != null && migrationReady) {
            AppNavigation(startDestination = startDestination, isLoggedIn = isLoggedIn)
        }
    }
}

private fun buildImageLoaderFactory(imageHttpClient: HttpClient): (PlatformContext) -> ImageLoader =
    { context: PlatformContext ->
        ImageLoader
            .Builder(context)
            .crossfade(150)
            .memoryCache {
                MemoryCache
                    .Builder()
                    .maxSizePercent(context, COIL_MEMORY_CACHE_MAX_SIZE_PERCENT)
                    .build()
            }.diskCache {
                DiskCache
                    .Builder()
                    .directory(coilDiskCachePath(context))
                    .maxSizeBytes(COIL_DISK_CACHE_MAX_SIZE_BYTES)
                    .build()
            }.components {
                add(ImgproxyCacheInterceptor())
                @OptIn(ExperimentalCoilApi::class)
                add(KtorNetworkFetcherFactory(imageHttpClient))
            }.build()
    }
