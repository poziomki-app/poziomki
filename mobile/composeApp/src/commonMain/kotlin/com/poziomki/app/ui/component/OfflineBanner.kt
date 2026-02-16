package com.poziomki.app.ui.component

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.data.connectivity.ConnectivityMonitor
import com.poziomki.app.ui.theme.Warning
import com.poziomki.app.ui.theme.White
import org.koin.compose.koinInject

@Composable
fun OfflineBanner(modifier: Modifier = Modifier) {
    val connectivityMonitor = koinInject<ConnectivityMonitor>()
    val isOnline by connectivityMonitor.isOnline.collectAsState()

    AnimatedVisibility(
        visible = !isOnline,
        enter = expandVertically(),
        exit = shrinkVertically(),
        modifier = modifier,
    ) {
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(Warning)
                    .padding(vertical = 6.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Offline — changes will sync when connected",
                color = White,
                fontSize = 12.sp,
                textAlign = TextAlign.Center,
            )
        }
    }
}
