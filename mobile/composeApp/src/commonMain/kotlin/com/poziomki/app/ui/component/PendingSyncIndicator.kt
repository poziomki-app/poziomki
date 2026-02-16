package com.poziomki.app.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.theme.White
import com.poziomki.app.data.sync.PendingOperationsManager
import org.koin.compose.koinInject

@Composable
fun PendingSyncIndicator(modifier: Modifier = Modifier) {
    val pendingOps = koinInject<PendingOperationsManager>()
    val pendingCount by pendingOps.observePendingCount().collectAsState(initial = 0L)

    if (pendingCount > 0) {
        Box(
            modifier =
                modifier
                    .size(18.dp)
                    .clip(CircleShape)
                    .background(Color(0xFFE65100)),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = if (pendingCount > 9) "9+" else pendingCount.toString(),
                color = White,
                fontSize = 10.sp,
            )
        }
    }
}
