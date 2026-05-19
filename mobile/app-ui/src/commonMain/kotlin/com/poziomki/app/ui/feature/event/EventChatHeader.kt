package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withLink
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.bold.Flag
import com.adamglin.phosphoricons.bold.Info
import com.adamglin.phosphoricons.bold.PencilSimple
import com.adamglin.phosphoricons.bold.SignOut
import com.adamglin.phosphoricons.bold.Trash
import com.adamglin.phosphoricons.bold.UserPlus
import com.adamglin.phosphoricons.fill.CalendarDots
import com.adamglin.phosphoricons.fill.MapPin
import com.poziomki.app.network.Event
import com.poziomki.app.network.EventAttendee
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.components.mapsDeeplink
import com.poziomki.app.ui.designsystem.components.rememberExternalLinkOpener
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.feature.chat.ActionMenuItem
import com.poziomki.app.ui.shared.formatEventDateCompact
import com.poziomki.app.ui.shared.formatEventLocation
import com.poziomki.app.ui.shared.resolveImageUrl

@Composable
fun EventCoverImage(
    event: Event,
    strongOverlay: Boolean = false,
    content: @Composable BoxScope.() -> Unit,
) {
    val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(start = 12.dp, end = 12.dp, top = statusBarTop + 8.dp)
                .aspectRatio(16f / 9f)
                .clip(RoundedCornerShape(24.dp)),
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

        val gradientStops =
            if (strongOverlay) {
                arrayOf(
                    0f to Color.Black.copy(alpha = 0.2f),
                    0.12f to Color.Transparent,
                    0.25f to Background.copy(alpha = 0.18f),
                    0.38f to Background.copy(alpha = 0.65f),
                    0.48f to Background,
                    1f to Background,
                )
            } else {
                arrayOf(
                    0f to Color.Black.copy(alpha = 0.18f),
                    0.15f to Color.Transparent,
                    0.55f to Color.Transparent,
                    0.72f to Background.copy(alpha = 0.55f),
                    0.88f to Background,
                    1f to Background,
                )
            }
        val solidBottomFraction = if (strongOverlay) 0.55f else 0.15f

        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(Brush.verticalGradient(colorStops = gradientStops)),
        )

        Box(
            modifier =
                Modifier
                    .align(Alignment.BottomStart)
                    .fillMaxWidth()
                    .fillMaxHeight(solidBottomFraction)
                    .background(Background),
        )

        content()
    }
}

@Composable
@Suppress("UnusedParameter")
fun EventMetaRows(
    event: Event,
    onParticipantsClick: (() -> Unit)? = null,
    onLocationClick: (() -> Unit)? = null,
    onInfoClick: (() -> Unit)? = null,
) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        MetaChip(icon = PhosphorIcons.Fill.CalendarDots, text = formatEventDateCompact(event.startsAt))

        event.location?.let { location ->
            MetaChip(
                icon = PhosphorIcons.Fill.MapPin,
                text = formatEventLocation(location),
                onClick = onLocationClick,
                preserveCase = true,
                modifier = Modifier.weight(1f, fill = false),
            )
        }

        if (onInfoClick != null) {
            Surface(
                onClick = onInfoClick,
                shape = CircleShape,
                color = Color.White.copy(alpha = 0.12f),
            ) {
                Box(
                    contentAlignment = Alignment.Center,
                    modifier = Modifier.size(32.dp),
                ) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.Info,
                        contentDescription = "informacje",
                        modifier = Modifier.size(18.dp),
                        tint = Color.White.copy(alpha = 0.85f),
                    )
                }
            }
        }
    }
}

@Composable
@Suppress("LongParameterList")
private fun MetaChip(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    text: String,
    onClick: (() -> Unit)? = null,
    accent: Boolean = false,
    preserveCase: Boolean = false,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(50)
    val bgColor = if (accent) Primary.copy(alpha = 0.18f) else Color.White.copy(alpha = 0.12f)
    val contentColor = if (accent) Primary else Color.White

    if (onClick != null) {
        Surface(onClick = onClick, shape = shape, color = bgColor, modifier = modifier) {
            ChipContent(icon = icon, text = text, tint = contentColor, preserveCase = preserveCase)
        }
    } else {
        Surface(shape = shape, color = bgColor, modifier = modifier) {
            ChipContent(icon = icon, text = text, tint = contentColor, preserveCase = preserveCase)
        }
    }
}

@Composable
private fun ChipContent(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    text: String,
    tint: Color,
    preserveCase: Boolean = false,
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier =
            Modifier
                .height(32.dp)
                .padding(horizontal = 10.dp),
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = tint.copy(alpha = 0.85f),
        )
        Spacer(modifier = Modifier.width(5.dp))
        Text(
            text = text,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Bold,
            fontSize = 12.sp,
            color = tint,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            preserveCase = preserveCase,
        )
    }
}

@Composable
@Suppress("LongMethod", "LongParameterList", "CyclomaticComplexMethod")
fun EventChatHeader(
    event: Event,
    attendees: List<EventAttendee>,
    isCreator: Boolean,
    onBack: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    onJoin: () -> Unit,
    onLeave: () -> Unit,
    onDelete: () -> Unit,
    onEdit: () -> Unit,
    onReport: () -> Unit = {},
) {
    var showMenu by remember { mutableStateOf(false) }
    var showDeleteDialog by remember { mutableStateOf(false) }
    var showAttendeesDialog by remember { mutableStateOf(false) }
    var showInfoDialog by remember { mutableStateOf(false) }
    val openLink = rememberExternalLinkOpener()

    EventCoverImage(event = event, strongOverlay = true) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopStart)
                    .padding(horizontal = 8.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                modifier = Modifier.size(40.dp),
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

            Spacer(modifier = Modifier.weight(1f))
            Box {
                Surface(
                    modifier = Modifier.size(40.dp),
                    shape = CircleShape,
                    color = Color.Black.copy(alpha = 0.45f),
                ) {
                    IconButton(onClick = { showMenu = true }) {
                        Icon(
                            imageVector = PhosphorIcons.Bold.DotsThreeVertical,
                            contentDescription = "Więcej",
                            tint = Color.White,
                            modifier = Modifier.size(22.dp),
                        )
                    }
                }
                DropdownMenu(
                    expanded = showMenu,
                    onDismissRequest = { showMenu = false },
                    shape = RoundedCornerShape(16.dp),
                    containerColor = SurfaceElevated,
                ) {
                    Column(modifier = Modifier.padding(horizontal = 4.dp)) {
                        if (isCreator) {
                            ActionMenuItem(
                                icon = PhosphorIcons.Bold.PencilSimple,
                                label = "Edytuj",
                                onClick = {
                                    showMenu = false
                                    onEdit()
                                },
                            )
                            ActionMenuItem(
                                icon = PhosphorIcons.Bold.Trash,
                                label = "Usuń wydarzenie",
                                onClick = {
                                    showMenu = false
                                    showDeleteDialog = true
                                },
                                iconTint = Error,
                                labelColor = Error,
                            )
                        } else if (event.isAttending) {
                            ActionMenuItem(
                                icon = PhosphorIcons.Bold.SignOut,
                                label = "Opuść wydarzenie",
                                onClick = {
                                    showMenu = false
                                    onLeave()
                                },
                            )
                        } else {
                            ActionMenuItem(
                                icon = PhosphorIcons.Bold.UserPlus,
                                label = "Dołącz do wydarzenia",
                                onClick = {
                                    showMenu = false
                                    onJoin()
                                },
                            )
                        }
                        ActionMenuItem(
                            icon = PhosphorIcons.Bold.Flag,
                            label = "Zgłoś",
                            onClick = {
                                showMenu = false
                                onReport()
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
                    .padding(
                        start = PoziomkiTheme.spacing.md,
                        end = PoziomkiTheme.spacing.md,
                        top = PoziomkiTheme.spacing.sm,
                        bottom = PoziomkiTheme.spacing.lg,
                    ),
        ) {
            val titleFontSize =
                when {
                    event.title.length > 40 -> 18.sp
                    event.title.length > 28 -> 22.sp
                    else -> 26.sp
                }
            Text(
                text = event.title,
                preserveCase = true,
                fontFamily = MontserratFamily,
                fontSize = titleFontSize,
                lineHeight = titleFontSize * 1.15f,
                fontWeight = FontWeight.ExtraBold,
                color = Color.White,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
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
                        preserveCase = true,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                        color = Primary,
                    )
                }
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))
            }

            EventMetaRows(
                event = event,
                onParticipantsClick = { showAttendeesDialog = true },
                onLocationClick =
                    event.latitude?.let { lat ->
                        event.longitude?.let { lng ->
                            {
                                openLink(mapsDeeplink(lat, lng, event.location))
                            }
                        }
                    },
                onInfoClick =
                    if (!event.description.isNullOrBlank()) {
                        { showInfoDialog = true }
                    } else {
                        null
                    },
            )
        }
    }

    if (showDeleteDialog) {
        ConfirmDialog(
            title = "usuń wydarzenie",
            message = "czy na pewno chcesz usunąć to wydarzenie? tej operacji nie można cofnąć.",
            confirmText = "usuń",
            isDestructive = true,
            onConfirm = {
                showDeleteDialog = false
                onDelete()
            },
            onDismiss = { showDeleteDialog = false },
        )
    }

    if (showAttendeesDialog) {
        AttendeesDialog(
            attendees = attendees,
            onDismiss = { showAttendeesDialog = false },
            onNavigateToProfile = { profileId ->
                showAttendeesDialog = false
                onNavigateToProfile(profileId)
            },
        )
    }

    if (showInfoDialog) {
        event.description?.let { description ->
            EventInfoDialog(
                description = description,
                onLinkClick = openLink,
                onDismiss = { showInfoDialog = false },
            )
        }
    }
}

@Composable
@Suppress("LongMethod")
private fun AttendeesDialog(
    attendees: List<EventAttendee>,
    onDismiss: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(16.dp),
        containerColor = SurfaceElevated,
        tonalElevation = 0.dp,
        title = {
            Text(
                text = "uczestnicy",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
                fontSize = 18.sp,
                color = TextPrimary,
            )
        },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
            ) {
                attendees.forEach { attendee ->
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .clickable { onNavigateToProfile(attendee.profileId) }
                                .padding(vertical = 6.dp),
                    ) {
                        UserAvatar(
                            picture = attendee.profilePicture,
                            displayName = attendee.name,
                            size = 36.dp,
                        )
                        Spacer(modifier = Modifier.width(12.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = attendee.name,
                                preserveCase = true,
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 14.sp,
                                color = TextPrimary,
                            )
                            if (attendee.isCreator) {
                                Text(
                                    text = "organizator",
                                    fontFamily = NunitoFamily,
                                    fontSize = 12.sp,
                                    color = Primary,
                                )
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(
                    text = "zamknij",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    color = TextMuted,
                )
            }
        },
    )
}

@Composable
private fun EventInfoDialog(
    description: String,
    onLinkClick: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(16.dp),
        containerColor = SurfaceElevated,
        tonalElevation = 0.dp,
        title = {
            Text(
                text = "opis",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
                fontSize = 18.sp,
                color = TextPrimary,
            )
        },
        text = {
            Text(
                text = linkifiedText(description, onLinkClick),
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Normal,
                fontSize = 14.sp,
                color = TextSecondary,
                lineHeight = 20.sp,
            )
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(
                    text = "zamknij",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    color = TextMuted,
                )
            }
        },
    )
}

private val urlRegex = Regex("https?://[\\w./?=&#%~_+\\-:@!$']+")

@Composable
private fun linkifiedText(
    text: String,
    onLinkClick: (String) -> Unit,
): AnnotatedString =
    buildAnnotatedString {
        var cursor = 0
        for (match in urlRegex.findAll(text)) {
            if (match.range.first > cursor) {
                append(text.substring(cursor, match.range.first))
            }
            val url = match.value
            val link =
                LinkAnnotation.Clickable(
                    tag = url,
                    styles =
                        TextLinkStyles(
                            style =
                                SpanStyle(
                                    color = Primary,
                                    textDecoration = TextDecoration.Underline,
                                ),
                        ),
                ) { onLinkClick(url) }
            withLink(link) { append(url) }
            cursor = match.range.last + 1
        }
        if (cursor < text.length) append(text.substring(cursor))
    }
