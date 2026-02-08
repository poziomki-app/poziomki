package com.poziomki.app.ui.screen.main

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary

@Composable
fun MessagesScreen(
    onNavigateToChat: (String) -> Unit,
    onNavigateToNewChat: () -> Unit,
) {
    Scaffold(
        containerColor = Background,
        floatingActionButton = {
            FloatingActionButton(
                onClick = onNavigateToNewChat,
                containerColor = Primary,
            ) {
                Icon(Icons.Filled.Edit, contentDescription = "Nowa wiadomo\u015b\u0107")
            }
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding),
        ) {
            // Header
            Text(
                text = "wiadomo\u015bci",
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
                modifier =
                    Modifier.padding(
                        horizontal = PoziomkiTheme.spacing.lg,
                        vertical = PoziomkiTheme.spacing.md,
                    ),
            )

            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    "czat b\u0119dzie nap\u0119dzany przez Matrix",
                    fontFamily = NunitoFamily,
                    color = TextSecondary,
                )
            }
        }
    }
}
