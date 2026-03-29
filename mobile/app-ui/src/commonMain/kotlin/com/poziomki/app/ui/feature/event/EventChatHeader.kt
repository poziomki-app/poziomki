package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.adamglin.phosphoricons.bold.Check
import com.adamglin.phosphoricons.bold.DotsThreeVertical
import com.adamglin.phosphoricons.bold.X
import com.adamglin.phosphoricons.fill.CalendarDots
import com.adamglin.phosphoricons.fill.MapPin
import com.adamglin.phosphoricons.fill.UsersThree
import com.poziomki.app.network.Event
import com.poziomki.app.network.EventAttendee
import com.poziomki.app.ui.designsystem.components.ConfirmDialog
import com.poziomki.app.ui.designsystem.components.UserAvatar
import com.poziomki.app.ui.designsystem.components.pointGeoJson
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.formatEventDateCompact
import com.poziomki.app.ui.shared.resolveImageUrl
import org.maplibre.compose.camera.CameraPosition
import org.maplibre.compose.camera.rememberCameraState
import org.maplibre.compose.expressions.dsl.const
import org.maplibre.compose.layers.CircleLayer
import org.maplibre.compose.map.MapOptions
import org.maplibre.compose.map.MaplibreMap
import org.maplibre.compose.map.OrnamentOptions
import org.maplibre.compose.sources.rememberGeoJsonSource
import org.maplibre.compose.style.BaseStyle
import org.maplibre.spatialk.geojson.Position

@Composable
fun EventCoverImage(
    event: Event,
    content: @Composable BoxScope.() -> Unit,
) {
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

        content()
    }
}

@Composable
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
                text = shortenLocation(location),
                onClick = onLocationClick,
            )
        }

        MetaChip(
            icon = PhosphorIcons.Fill.UsersThree,
            text = event.attendeesCount.toString(),
            onClick = onParticipantsClick,
            accent = true,
        )

        if (onInfoClick != null) {
            val infoShape = RoundedCornerShape(50)
            Surface(
                onClick = onInfoClick,
                shape = infoShape,
                color = Primary.copy(alpha = 0.18f),
            ) {
                Box(
                    contentAlignment = Alignment.Center,
                    modifier =
                        Modifier
                            .height(32.dp)
                            .padding(horizontal = 12.dp),
                ) {
                    Text(
                        text = "i",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 14.sp,
                        color = Primary,
                    )
                }
            }
        }
    }
}

@Composable
private fun MetaChip(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    text: String,
    onClick: (() -> Unit)? = null,
    accent: Boolean = false,
) {
    val shape = RoundedCornerShape(50)
    val bgColor = if (accent) Primary.copy(alpha = 0.18f) else Color.White.copy(alpha = 0.12f)
    val contentColor = if (accent) Primary else Color.White

    if (onClick != null) {
        Surface(onClick = onClick, shape = shape, color = bgColor) {
            ChipContent(icon = icon, text = text, tint = contentColor)
        }
    } else {
        Surface(shape = shape, color = bgColor) {
            ChipContent(icon = icon, text = text, tint = contentColor)
        }
    }
}

@Composable
private fun ChipContent(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    text: String,
    tint: Color,
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
        )
    }
}

private fun shortenLocation(location: String): String {
    val parts = location.split(",").map { it.trim() }
    if (parts.size <= 2) return location
    return parts.dropLast(1).joinToString(", ")
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
    onApprove: (String) -> Unit = {},
    onReject: (String) -> Unit = {},
) {
    var showMenu by remember { mutableStateOf(false) }
    var showDeleteDialog by remember { mutableStateOf(false) }
    var showAttendeesDialog by remember { mutableStateOf(false) }
    var showLocationDialog by remember { mutableStateOf(false) }
    var showInfoDialog by remember { mutableStateOf(false) }

    EventCoverImage(event = event) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopStart)
                    .statusBarsPadding()
                    .padding(horizontal = 4.dp, vertical = 4.dp),
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
                ) {
                    if (isCreator) {
                        DropdownMenuItem(
                            text = { Text("Edytuj") },
                            onClick = {
                                showMenu = false
                                onEdit()
                            },
                        )
                        DropdownMenuItem(
                            text = {
                                Text(
                                    "Usuń wydarzenie",
                                    color = MaterialTheme.colorScheme.error,
                                )
                            },
                            onClick = {
                                showMenu = false
                                showDeleteDialog = true
                            },
                        )
                    } else if (event.isAttending) {
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

            EventMetaRows(
                event = event,
                onParticipantsClick = { showAttendeesDialog = true },
                onLocationClick =
                    if (event.latitude != null && event.longitude != null) {
                        { showLocationDialog = true }
                    } else {
                        null
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
            isCreator = isCreator,
            onDismiss = { showAttendeesDialog = false },
            onNavigateToProfile = { profileId ->
                showAttendeesDialog = false
                onNavigateToProfile(profileId)
            },
            onApprove = onApprove,
            onReject = onReject,
        )
    }

    if (showLocationDialog) {
        val lat = event.latitude
        val lng = event.longitude
        val loc = event.location
        if (lat != null && lng != null && loc != null) {
            LocationMapDialog(
                locationName = loc,
                latitude = lat,
                longitude = lng,
                onDismiss = { showLocationDialog = false },
            )
        }
    }

    if (showInfoDialog) {
        event.description?.let { description ->
            EventInfoDialog(
                description = description,
                onDismiss = { showInfoDialog = false },
            )
        }
    }
}

@Composable
@Suppress("LongMethod", "LongParameterList")
private fun AttendeesDialog(
    attendees: List<EventAttendee>,
    isCreator: Boolean,
    onDismiss: () -> Unit,
    onNavigateToProfile: (String) -> Unit,
    onApprove: (String) -> Unit,
    onReject: (String) -> Unit,
) {
    val pending = attendees.filter { it.status == "pending" }
    val confirmed = attendees.filter { it.status != "pending" }

    AlertDialog(
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(16.dp),
        containerColor = SurfaceElevated,
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
                if (pending.isNotEmpty() && isCreator) {
                    Text(
                        text = "oczekujący",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 12.sp,
                        color = TextMuted,
                        modifier = Modifier.padding(bottom = 4.dp),
                    )
                    pending.forEach { attendee ->
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .padding(vertical = 6.dp),
                        ) {
                            UserAvatar(
                                picture = attendee.profilePicture,
                                displayName = attendee.name,
                                size = 36.dp,
                                modifier = Modifier.clickable { onNavigateToProfile(attendee.profileId) },
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text(
                                text = attendee.name,
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 14.sp,
                                color = TextPrimary,
                                modifier = Modifier.weight(1f).clickable { onNavigateToProfile(attendee.profileId) },
                            )
                            IconButton(onClick = { onApprove(attendee.profileId) }) {
                                Icon(
                                    PhosphorIcons.Bold.Check,
                                    contentDescription = "Zatwierdź",
                                    tint = Color(0xFF4CAF50),
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                            IconButton(onClick = { onReject(attendee.profileId) }) {
                                Icon(
                                    PhosphorIcons.Bold.X,
                                    contentDescription = "Odrzuć",
                                    tint = Color(0xFFE57373),
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                        }
                    }
                    if (confirmed.isNotEmpty()) {
                        Spacer(modifier = Modifier.height(12.dp))
                    }
                }
                confirmed.forEach { attendee ->
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
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(16.dp),
        containerColor = SurfaceElevated,
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
                text = description,
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

@Composable
@Suppress("LongMethod")
private fun LocationMapDialog(
    locationName: String,
    latitude: Double,
    longitude: Double,
    onDismiss: () -> Unit,
) {
    val uriHandler = LocalUriHandler.current
    val mapsUrl = "https://www.google.com/maps/search/?api=1&query=$latitude,$longitude"

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(16.dp),
            color = SurfaceElevated,
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = locationName,
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp,
                    color = TextPrimary,
                    maxLines = 2,
                )
                Spacer(modifier = Modifier.height(12.dp))
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .height(220.dp)
                            .clip(RoundedCornerShape(12.dp)),
                ) {
                    MaplibreMap(
                        modifier = Modifier.fillMaxSize(),
                        baseStyle =
                            BaseStyle.Uri("https://tiles.openfreemap.org/styles/positron"),
                        cameraState =
                            rememberCameraState(
                                firstPosition =
                                    CameraPosition(
                                        target = Position(latitude = latitude, longitude = longitude),
                                        zoom = 14.0,
                                    ),
                            ),
                        options =
                            MapOptions(
                                ornamentOptions =
                                    OrnamentOptions(
                                        isLogoEnabled = false,
                                        isCompassEnabled = false,
                                        isScaleBarEnabled = false,
                                    ),
                            ),
                    ) {
                        val source = rememberGeoJsonSource(data = pointGeoJson(latitude, longitude))
                        CircleLayer(
                            id = "location-marker",
                            source = source,
                            radius = const(8.dp),
                            color = const(Primary),
                            strokeColor = const(Color.White),
                            strokeWidth = const(2.dp),
                        )
                    }
                }
                Spacer(modifier = Modifier.height(12.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TextButton(onClick = { uriHandler.openUri(mapsUrl) }) {
                        Icon(
                            imageVector = PhosphorIcons.Fill.MapPin,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = Primary,
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = "otwórz w mapach",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.SemiBold,
                            color = Primary,
                        )
                    }
                    TextButton(onClick = onDismiss) {
                        Text(
                            text = "zamknij",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.SemiBold,
                            color = TextMuted,
                        )
                    }
                }
            }
        }
    }
}
