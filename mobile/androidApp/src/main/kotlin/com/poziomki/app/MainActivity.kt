package com.poziomki.app

import android.content.Intent
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.LaunchedEffect
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.poziomki.app.chat.push.NotificationChatTarget
import com.poziomki.app.session.AppPreferences
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {
    private val appPreferences: AppPreferences by inject()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Block screenshots + recent-apps previews by default. Chat
        // messages, profile edit, and password-reset flows all surface
        // PII that shouldn't leak to a device's recents carousel,
        // screen-record apps, or untrusted projections. Users can opt
        // in via Privacy settings; the toggle defaults off so the first
        // frame is always protected.
        applySecureFlag(secure = true)
        // Wrap defensively at the coroutine boundary — a failure to
        // read the privacy flag must never crash the activity. Worst
        // case the user stays on the safe default (FLAG_SECURE on).
        lifecycleScope.launch {
            try {
                repeatOnLifecycle(Lifecycle.State.STARTED) {
                    appPreferences.screenshotsAllowed.collect { allowed ->
                        applySecureFlag(secure = !allowed)
                    }
                }
            } catch (
                @Suppress("TooGenericExceptionCaught") t: Throwable,
            ) {
                android.util.Log.e("MainActivity", "screenshotsAllowed observer crashed", t)
            }
        }
        if (savedInstanceState == null) {
            handleIntent(intent)
        }
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.BLACK),
        )
        setContent {
            // Remove when https://issuetracker.google.com/issues/364713509 is fixed
            LaunchedEffect(Unit) {
                enableEdgeToEdge(
                    statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
                    navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.BLACK),
                )
            }
            App()
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        NotificationChatTarget.open(intent?.getStringExtra(NotificationChatTarget.EXTRA_OPEN_CHAT_ROOM_ID))
    }

    private fun applySecureFlag(secure: Boolean) {
        if (secure) {
            window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
    }
}
