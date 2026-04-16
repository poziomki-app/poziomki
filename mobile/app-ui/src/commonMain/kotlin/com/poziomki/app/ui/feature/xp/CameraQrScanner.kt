package com.poziomki.app.ui.feature.xp

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
expect fun CameraQrScanner(
    onScanned: (String) -> Unit,
    modifier: Modifier = Modifier,
)
