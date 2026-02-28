package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.poziomki.app.network.Event
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.formatEventDateFull
import com.poziomki.app.ui.shared.pluralizePolish
import com.poziomki.app.ui.shared.resolveImageUrl
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.fill.CalendarDots
import com.adamglin.phosphoricons.fill.MapPin
import com.adamglin.phosphoricons.fill.UsersThree

@Composable
@Suppress("LongMethod", "LongParameterList")
fun EventChatHeader(
    event: Event,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    onJoin: () -> Unit,
    onLeave: () -> Unit,
) {
    var showMenu by remember { mutableStateOf(false) }

    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .aspectRatio(16f / 10f),
    ) {
        val coverImage = event.coverImage
        if (coverImage != null) {
            AsyncImage(
                model = resolveImageUrl(coverImage),
                contentDescription = event.title,
                modifier = Modifier.fillMaxSize(),
                contentScale = ContentScale.Crop,
            )
        }

        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(
                        Brush.verticalGradient(
                            colorStops =
                                arrayOf(
                                    0f to Color.Black.copy(alpha = 0.3f),
                                    0.2f to Color.Transparent,
                                    0.45f to Background.copy(alpha = 0.3f),
                                    0.65f to Background.copy(alpha = 0.65f),
                                    0.8f to Background.copy(alpha = 0.85f),
                                    1f to Background,
                                ),
                        ),
                    ),
        )

        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopStart)
                    .statusBarsPadding()
                    .padding(horizontal = 4.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    imageVector = PhosphorIcons.Bold.ArrowLeft,
                    contentDescription = "Wstecz",
                    tint = Color.White,
                )
            }

            Spacer(modifier = Modifier.weight(1f))
            Box {
                IconButton(onClick = { showMenu = true }) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.DotsThreeVertical,
                        contentDescription = "Więcej",
                        tint = Color.White,
                    )
                }
                DropdownMenu(
                    expanded = showMenu,
                    onDismissRequest = { showMenu = false },
                ) {
                    if (event.isAttending) {
                        DropdownMenuItem(
                            text = { Text("Opuść wydarzenie") },
                            onClick = {
                                showMenu = false
                                onLeave()
                            },
                        )
                    } else {
                        DropdownMenuItem(
                            text = { Text("Dołącz do wydarzenia") },
                            onClick = {
                                showMenu = false
                                onJoin()
                            },
                        )
                    }
                }
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
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.ExtraBold,
                color = Color.White,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))

            event.creator?.let { creator ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.clickable { onNavigateToProfile(creator.id) },
                ) {
                    UserAvatar(
                        picture = creator.profilePicture,
                        displayName = creator.name,
                        size = 36.dp,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = creator.name,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                        color = Primary,
                    )
                }
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))
            }

            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = PhosphorIcons.Fill.CalendarDots,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = TextSecondary,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = formatEventDateFull(event.startsAt),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                )
            }

            event.location?.let { location ->
                Spacer(modifier = Modifier.height(2.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = PhosphorIcons.Fill.MapPin,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp),
                        tint = TextSecondary,
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = location,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                        maxLines = 1,
                    )
                }
            }

            Spacer(modifier = Modifier.height(2.dp))

            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = PhosphorIcons.Fill.UsersThree,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = TextSecondary,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text =
                        pluralizePolish(
                            event.attendeesCount,
                            "uczestnik",
                            "uczestników",
                            "uczestników",
                        ),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                )
            }
        }
    }
}
