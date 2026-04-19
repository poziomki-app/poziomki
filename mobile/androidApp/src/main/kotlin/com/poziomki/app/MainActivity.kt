package com.poziomki.app

import android.content.Intent
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.LaunchedEffect
import com.poziomki.app.chat.push.NotificationChatTarget

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Block screenshots + recent-apps previews of the whole app.
        // Chat messages, profile edit, and password-reset flows all
        // surface PII that shouldn't leak to a device's recents
        // carousel, screen-record apps, or untrusted projections.
        // Set once on MainActivity because the app is single-activity;
        // every screen inherits the window flag.
        window.setFlags(
            WindowManager.LayoutParams.FLAG_SECURE,
            WindowManager.LayoutParams.FLAG_SECURE,
        )
        if (savedInstanceState == null) {
            handleIntent(intent)
        }
        enableEdgeToEdge()
        setContent {
            // Remove when https://issuetracker.google.com/issues/364713509 is fixed
            LaunchedEffect(Unit) {
                enableEdgeToEdge()
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
}
