package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.poziomki.app.network.Event
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.components.mapsDeeplink
import com.poziomki.app.ui.designsystem.components.rememberExternalLinkOpener
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
fun EventChatLoadingView(onBack: () -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding(),
    ) {
        IconButton(
            onClick = onBack,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.ArrowLeft,
                contentDescription = "Wstecz",
                tint = TextPrimary,
            )
        }
        CircularProgressIndicator(color = Primary, modifier = Modifier.align(Alignment.Center))
    }
}

@Composable
fun EventChatErrorView(
    message: String,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding(),
    ) {
        IconButton(
            onClick = onBack,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.ArrowLeft,
                contentDescription = "Wstecz",
                tint = TextPrimary,
            )
        }
        Column(
            modifier = Modifier.align(Alignment.Center).padding(horizontal = 24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = message,
                color = TextSecondary,
                fontFamily = NunitoFamily,
                fontSize = 15.sp,
            )
            Spacer(Modifier.height(16.dp))
            AppButton(
                text = "Spróbuj ponownie",
                onClick = onRetry,
                variant = ButtonVariant.PRIMARY,
            )
        }
    }
}

@Composable
fun EventChatNotFoundView(onBack: () -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding(),
    ) {
        IconButton(
            onClick = onBack,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.ArrowLeft,
                contentDescription = "Wstecz",
                tint = TextPrimary,
            )
        }
        Text("Nie znaleziono wydarzenia", color = TextSecondary, modifier = Modifier.align(Alignment.Center))
    }
}

@Composable
@Suppress("LongMethod")
fun EventChatJoinRequiredView(
    event: Event,
    isUpdatingAttendance: Boolean,
    onJoin: () -> Unit,
    onBack: () -> Unit,
) {
    val openLink = rememberExternalLinkOpener()
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState()),
    ) {
        EventCoverImage(event = event) {
            Surface(
                modifier =
                    Modifier
                        .align(Alignment.TopStart)
                        .padding(horizontal = 8.dp, vertical = 8.dp)
                        .size(40.dp),
                shape = CircleShape,
                color = Color.Black.copy(alpha = 0.45f),
            ) {
                IconButton(onClick = onBack) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.ArrowLeft,
                        contentDescription = "Wstecz",
                        tint = Color.White,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }

            Column(
                modifier =
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm),
            ) {
                Text(
                    text = event.title,
                    preserveCase = true,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.ExtraBold,
                    color = Color.White,
                    maxLines = 3,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }

        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.md),
        ) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            EventMetaRows(
                event = event,
                onLocationClick =
                    event.latitude?.let { lat ->
                        event.longitude?.let { lng ->
                            {
                                openLink(mapsDeeplink(lat, lng, event.location))
                            }
                        }
                    },
            )

            if (event.requiresApproval) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "wymaga akceptacji organizatora",
                    fontFamily = NunitoFamily,
                    fontSize = 13.sp,
                    color = TextMuted,
                )
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (event.isPending) {
                    JoinPillButton(
                        text = "oczekuje na akceptację",
                        onClick = {},
                        enabled = false,
                    )
                } else {
                    JoinPillButton(
                        text = "dołącz",
                        onClick = onJoin,
                        loading = isUpdatingAttendance,
                    )
                }
                if (event.attendeesPreview.isNotEmpty()) {
                    Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.md))
                    AttendeesCluster(
                        previews = event.attendeesPreview,
                        totalCount = event.attendeesCount,
                    )
                }
            }

            event.description?.let { description ->
                if (description.isNotBlank()) {
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))
                    Text(
                        text = "o wydarzeniu",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))
                    Text(
                        text = description,
                        preserveCase = true,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        }
    }
}

@Composable
private fun AttendeesCluster(
    previews: List<com.poziomki.app.network.EventAttendeePreview>,
    totalCount: Int,
    maxAvatars: Int = 3,
) {
    val avatarSize = 32.dp
    val overlapOffset = (-10).dp
    val step = avatarSize + overlapOffset
    val shown = previews.take(maxAvatars)
    val overflow = (totalCount - shown.size).coerceAtLeast(0)
    val slots = shown.size + if (overflow > 0) 1 else 0
    val totalWidth = if (slots == 0) 0.dp else avatarSize + (step * (slots - 1))

    Box(modifier = Modifier.size(width = totalWidth, height = avatarSize)) {
        shown.forEachIndexed { index, preview ->
            UserAvatar(
                picture = preview.profilePicture,
                displayName = preview.name,
                size = avatarSize,
                modifier =
                    Modifier
                        .offset(x = step * index)
                        .border(2.dp, Color(0xFF0B0F14), CircleShape),
            )
        }
        if (overflow > 0) {
            Box(
                modifier =
                    Modifier
                        .offset(x = step * shown.size)
                        .size(avatarSize)
                        .clip(CircleShape)
                        .background(Color(0xFFF2F4F7))
                        .border(2.dp, Color(0xFF0B0F14), CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "+$overflow",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                    fontSize = 12.sp,
                    color = Color(0xFF0B0F14),
                )
            }
        }
    }
}

@Composable
internal fun JoinPillButton(
    text: String,
    onClick: () -> Unit,
    enabled: Boolean = true,
    loading: Boolean = false,
) {
    val isEnabled = enabled && !loading
    val fill = Color(0xFFF2F4F7)
    val contentColor = Color(0xFF0B0F14).let { if (isEnabled) it else it.copy(alpha = 0.35f) }
    Row(
        modifier =
            Modifier
                .clip(RoundedCornerShape(50))
                .background(fill)
                .then(if (isEnabled) Modifier.clickable(onClick = onClick) else Modifier)
                .padding(horizontal = 22.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (loading) {
            CircularProgressIndicator(
                modifier = Modifier.size(16.dp),
                color = contentColor,
                strokeWidth = 2.dp,
            )
            Spacer(modifier = Modifier.width(8.dp))
        }
        Text(
            text = text,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.SemiBold,
            fontSize = 15.sp,
            color = contentColor,
        )
    }
}
