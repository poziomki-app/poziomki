package com.poziomki.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import coil3.network.ktor3.KtorNetworkFetcherFactory
import com.poziomki.app.data.sync.SyncEngine
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.navigation.AppNavigation
import com.poziomki.app.ui.navigation.Route
import com.poziomki.app.ui.theme.PoziomkiTheme
import io.ktor.client.HttpClient
import io.ktor.client.engine.HttpClientEngine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.koin.compose.koinInject

@Composable
fun App() {
    val engine = koinInject<HttpClientEngine>()

    setSingletonImageLoaderFactory { context ->
        ImageLoader
            .Builder(context)
            .components {
                add(KtorNetworkFetcherFactory(HttpClient(engine)))
            }.build()
    }

    val sessionManager = koinInject<SessionManager>()
    val syncEngine = koinInject<SyncEngine>()
    val isLoggedIn by sessionManager.isLoggedIn.collectAsState(
        initial = runBlocking { sessionManager.isLoggedIn.first() },
    )

    // Compute start destination once from initial session state.
    // Not reactive — sign-up saving a session mid-flow must NOT change startDestination.
    val startDestination = remember {
        val hasUser = runBlocking { sessionManager.isLoggedIn.first() }
        val hasProfile = runBlocking { sessionManager.profileId.first() } != null
        when {
            !hasUser -> Route.AuthGraph
            !hasProfile -> Route.OnboardingGraph
            else -> Route.MainGraph
        }
    }

    DisposableEffect(Unit) {
        syncEngine.start()
        onDispose { syncEngine.stop() }
    }

    PoziomkiTheme {
        AppNavigation(startDestination = startDestination, isLoggedIn = isLoggedIn)
    }
}
