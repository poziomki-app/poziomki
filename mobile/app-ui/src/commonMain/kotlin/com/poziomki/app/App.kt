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
import com.poziomki.app.chat.api.ChatClient
import com.poziomki.app.data.sync.SyncEngine
import com.poziomki.app.session.SessionBootstrapState
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.navigation.AppNavigation
import com.poziomki.app.ui.navigation.Route
import com.poziomki.app.ui.shared.ImgproxyCacheInterceptor
import com.poziomki.app.ui.shared.MxcMediaFetcher
import io.ktor.client.HttpClient
import io.ktor.client.engine.HttpClientEngine
import okio.Path.Companion.toOkioPath
import org.koin.compose.koinInject

private const val COIL_MEMORY_CACHE_MAX_SIZE_PERCENT = 0.18
private const val COIL_DISK_CACHE_MAX_SIZE_BYTES = 48L * 1024L * 1024L

@Composable
fun App() {
    val engine = koinInject<HttpClientEngine>()
    val chatClient = koinInject<ChatClient>()
    val imageHttpClient = remember(engine) { HttpClient(engine) }
    val imageLoaderFactory: (PlatformContext) -> ImageLoader =
        remember(imageHttpClient) { buildImageLoaderFactory(imageHttpClient) }

    setSingletonImageLoaderFactory(imageLoaderFactory)

    val sessionManager = koinInject<SessionManager>()
    val syncEngine = koinInject<SyncEngine>()
    val bootstrapState by
        produceState<SessionBootstrapState?>(initialValue = null, sessionManager) {
            value = sessionManager.getBootstrapState()
        }
    val isLoggedIn by sessionManager.isLoggedIn.collectAsState(initial = bootstrapState?.isLoggedIn ?: false)

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

    DisposableEffect(Unit) {
        syncEngine.start()
        onDispose {
            syncEngine.stop()
            imageHttpClient.close()
        }
    }

    // Warm up chat client in background so first chat open is fast.
    LaunchedEffect(startDestination) {
        if (startDestination == Route.MainGraph) {
            chatClient.ensureStarted()
        }
    }

    PoziomkiTheme {
        if (startDestination != null) {
            AppNavigation(startDestination = startDestination, isLoggedIn = isLoggedIn)
        }
    }
}

private fun buildImageLoaderFactory(
    imageHttpClient: HttpClient,
): (PlatformContext) -> ImageLoader =
    { context: PlatformContext ->
        ImageLoader
            .Builder(context)
            .memoryCache {
                MemoryCache
                    .Builder()
                    .maxSizePercent(context, COIL_MEMORY_CACHE_MAX_SIZE_PERCENT)
                    .build()
            }.diskCache {
                DiskCache
                    .Builder()
                    .directory(context.cacheDir.resolve("coil_images").toOkioPath())
                    .maxSizeBytes(COIL_DISK_CACHE_MAX_SIZE_BYTES)
                    .build()
            }.components {
                add(ImgproxyCacheInterceptor())
                add(MxcMediaFetcher.Factory())
                @OptIn(ExperimentalCoilApi::class)
                add(KtorNetworkFetcherFactory(imageHttpClient))
            }.build()
    }
