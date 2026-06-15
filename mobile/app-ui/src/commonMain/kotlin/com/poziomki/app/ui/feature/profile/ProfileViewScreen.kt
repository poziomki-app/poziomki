package com.poziomki.app.ui.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.BookmarkSimple
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.bold.Flag
import com.adamglin.phosphoricons.bold.Prohibit
import com.adamglin.phosphoricons.bold.X
import com.adamglin.phosphoricons.fill.BookmarkSimple
import com.adamglin.phosphoricons.fill.PaperPlaneRight
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.ProfileImage
import com.poziomki.app.ui.designsystem.components.ProfilePreview
import com.poziomki.app.ui.designsystem.components.ReportDialog
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.feature.chat.ActionMenuItem
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
    var showMenu by remember { mutableStateOf(false) }
    var showReportDialog by remember { mutableStateOf(false) }
    var showBlockConfirm by remember { mutableStateOf(false) }

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

                val statusBarTop =
                    WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
                Box(modifier = Modifier.fillMaxSize()) {
                    val displayedProgram =
                        if (state.isOwnProfile && !state.showOwnProgram) null else p.program
                    ProfilePreview(
                        name = p.name,
                        program = displayedProgram,
                        bio = p.bio,
                        tags = p.tags,
                        images = images,
                        emojiAvatar = emoji,
                        gradientStart = p.gradientStart,
                        gradientEnd = p.gradientEnd,
                        ownTagIds = if (state.isOwnProfile) emptySet() else state.ownTagIds,
                        // X is rendered as a sticky overlay below; tell
                        // ProfilePreview not to draw its own (which would
                        // scroll with the image carousel).
                        onClose = null,
                        headerAction =
                            if (!state.isOwnProfile) {
                                {
                                    Row {
                                        IconButton(
                                            onClick = {
                                                onNavigateToChat(p.userId, p.name, p.id)
                                            },
                                        ) {
                                            Icon(
                                                imageVector = PhosphorIcons.Fill.PaperPlaneRight,
                                                contentDescription = "Napisz wiadomość",
                                                tint = Primary,
                                                modifier = Modifier.size(22.dp),
                                            )
                                        }
                                        IconButton(onClick = { viewModel.toggleBookmark() }) {
                                            Icon(
                                                imageVector =
                                                    if (state.isBookmarked) {
                                                        PhosphorIcons.Fill.BookmarkSimple
                                                    } else {
                                                        PhosphorIcons.Bold.BookmarkSimple
                                                    },
                                                contentDescription =
                                                    if (state.isBookmarked) {
                                                        "Usuń zakładkę"
                                                    } else {
                                                        "Dodaj zakładkę"
                                                    },
                                                tint = if (state.isBookmarked) Primary else White,
                                                modifier = Modifier.size(22.dp),
                                            )
                                        }
                                        Box {
                                            IconButton(onClick = { showMenu = true }) {
                                                Icon(
                                                    imageVector = PhosphorIcons.Bold.DotsThreeVertical,
                                                    contentDescription = "Więcej",
                                                    tint = White,
                                                    modifier = Modifier.size(22.dp),
                                                )
                                            }
                                            DropdownMenu(
                                                expanded = showMenu,
                                                onDismissRequest = { showMenu = false },
                                                shape = RoundedCornerShape(16.dp),
                                                containerColor = SurfaceElevated,
                                            ) {
                                                ActionMenuItem(
                                                    icon = PhosphorIcons.Bold.Flag,
                                                    label = "Zgłoś",
                                                    onClick = {
                                                        showMenu = false
                                                        showReportDialog = true
                                                    },
                                                )
                                                ActionMenuItem(
                                                    icon = PhosphorIcons.Bold.Prohibit,
                                                    label = "Zablokuj",
                                                    onClick = {
                                                        showMenu = false
                                                        showBlockConfirm = true
                                                    },
                                                    iconTint = Error,
                                                    labelColor = Error,
                                                )
                                            }
                                        }
                                    }
                                }
                            } else {
                                null
                            },
                    )

                    // Sticky X (close) — top-end, inset from the image's
                    // rounded corner so it never overlaps the curved edge.
                    // Image: 12dp horizontal padding + 24dp corner radius,
                    // top at statusBarTop + 8dp.
                    IconButton(
                        onClick = onBack,
                        modifier =
                            Modifier
                                .align(Alignment.TopEnd)
                                .padding(top = statusBarTop + 16.dp, end = 20.dp)
                                .size(40.dp)
                                .clip(CircleShape)
                                .background(Color.Black.copy(alpha = 0.45f)),
                    ) {
                        Icon(
                            imageVector = PhosphorIcons.Bold.X,
                            contentDescription = "Zamknij",
                            tint = White,
                            modifier = Modifier.size(24.dp),
                        )
                    }

                    if (showReportDialog) {
                        ReportDialog(
                            onConfirm = { reason, _ ->
                                showReportDialog = false
                                viewModel.reportUser(reason)
                            },
                            onDismiss = { showReportDialog = false },
                        )
                    }

                    if (showBlockConfirm) {
                        ConfirmDialog(
                            title = "zablokuj użytkownika",
                            message = "nie będziesz już widzieć tej osoby ani jej treści. zgłosimy ją do moderacji.",
                            confirmText = "zablokuj",
                            isDestructive = true,
                            onConfirm = {
                                showBlockConfirm = false
                                viewModel.blockUser(onBlocked = onBack)
                            },
                            onDismiss = { showBlockConfirm = false },
                        )
                    }
                }
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
