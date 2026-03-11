package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.designsystem.components.EmptyView
import com.poziomki.app.ui.designsystem.components.ScreenHeader

@Composable
fun GroupsScreen(profileAvatarAction: @Composable () -> Unit = {}) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        ScreenHeader(title = "grupy") {
            profileAvatarAction()
        }
        EmptyView("wkr\u00f3tce")
    }
}
