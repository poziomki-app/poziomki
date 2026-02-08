package com.poziomki.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import coil3.network.ktor3.KtorNetworkFetcherFactory
import com.poziomki.app.session.SessionManager
import com.poziomki.app.ui.navigation.AppNavigation
import com.poziomki.app.ui.navigation.Route
import com.poziomki.app.ui.theme.PoziomkiTheme
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.koin.compose.koinInject

@Composable
fun App() {
    setSingletonImageLoaderFactory { context ->
        ImageLoader
            .Builder(context)
            .components {
                add(KtorNetworkFetcherFactory())
            }.build()
    }

    val sessionManager = koinInject<SessionManager>()
    val startDestination =
        remember {
            val loggedIn = runBlocking { sessionManager.isLoggedIn.first() }
            if (loggedIn) Route.MainGraph else Route.AuthGraph
        }

    PoziomkiTheme {
        AppNavigation(startDestination = startDestination)
    }
}
