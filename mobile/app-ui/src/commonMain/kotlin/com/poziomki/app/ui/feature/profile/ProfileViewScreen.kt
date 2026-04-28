package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.BookmarkSimple
import com.adamglin.phosphoricons.fill.BookmarkSimple
import com.adamglin.phosphoricons.fill.PaperPlaneRight
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.ProfileImage
import com.poziomki.app.ui.designsystem.components.ProfilePreview
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.shared.isImageUrl
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@Composable
@Suppress("LongMethod", "CyclomaticComplexMethod")
fun ProfileViewScreen(
    onBack: () -> Unit,
    onNavigateToChat: (String, String, String?) -> Unit,
    viewModel: ProfileViewViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    when {
        state.profile != null -> {
            state.profile?.let { p ->
                val images =
                    buildList {
                        p.profilePicture?.let { if (isImageUrl(it)) add(ProfileImage.Url(it)) }
                        p.images
                            .filter { isImageUrl(it) && it != p.profilePicture }
                            .forEach { add(ProfileImage.Url(it)) }
                    }
                val emoji = p.profilePicture?.takeUnless { isImageUrl(it) }

                val bottomInsets =
                    WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
                ProfilePreview(
                    name = p.name,
                    program = p.program,
                    bio = p.bio,
                    tags = p.tags,
                    images = images,
                    emojiAvatar = emoji,
                    gradientStart = p.gradientStart,
                    gradientEnd = p.gradientEnd,
                    onClose = onBack,
                    headerAction =
                        if (!state.isOwnProfile) {
                            {
                                IconButton(onClick = { viewModel.toggleBookmark() }) {
                                    Icon(
                                        imageVector =
                                            if (state.isBookmarked) {
                                                PhosphorIcons.Fill.BookmarkSimple
                                            } else {
                                                PhosphorIcons.Bold.BookmarkSimple
                                            },
                                        contentDescription =
                                            if (state.isBookmarked) "Usuń zakładkę" else "Dodaj zakładkę",
                                        tint = if (state.isBookmarked) Primary else White,
                                        modifier = Modifier.size(22.dp),
                                    )
                                }
                            }
                        } else {
                            null
                        },
                    bottomContent =
                        if (!state.isOwnProfile) {
                            {
                                Box(
                                    modifier =
                                        Modifier
                                            .fillMaxWidth()
                                            .padding(
                                                start = 16.dp,
                                                end = 16.dp,
                                                top = 12.dp,
                                                bottom = bottomInsets + 20.dp,
                                            ),
                                    contentAlignment = Alignment.CenterEnd,
                                ) {
                                    AppButton(
                                        text = "Wiadomość",
                                        onClick = { onNavigateToChat(p.userId, p.name, p.id) },
                                        variant = ButtonVariant.PRIMARY,
                                        icon = PhosphorIcons.Fill.PaperPlaneRight,
                                    )
                                }
                            }
                        } else {
                            null
                        },
                )
            }
        }

        state.isLoading -> {
            var showSpinner by remember { mutableStateOf(false) }
            LaunchedEffect(Unit) {
                delay(300)
                showSpinner = true
            }
            Box(Modifier.fillMaxSize().background(Background), contentAlignment = Alignment.Center) {
                if (showSpinner) CircularProgressIndicator(color = Primary)
            }
        }

        else -> {
            Box(Modifier.fillMaxSize().background(Background), contentAlignment = Alignment.Center) {
                Text(
                    text = "nie znaleziono profilu",
                    fontFamily = NunitoFamily,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
    }
}
