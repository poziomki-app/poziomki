package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.MapPinLine
import com.poziomki.app.network.Event
import com.poziomki.app.network.GeocodingService
import com.poziomki.app.ui.designsystem.components.StackedAvatars
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.designsystem.theme.White
import com.poziomki.app.ui.navigation.LocalNavBarPadding
import com.poziomki.app.ui.shared.formatEventDate
import com.poziomki.app.ui.shared.pluralizePolish
import com.poziomki.app.ui.shared.resolveImageUrl
import org.koin.compose.koinInject
import org.maplibre.compose.camera.CameraPosition
import org.maplibre.compose.camera.rememberCameraState
import org.maplibre.compose.expressions.dsl.const
import org.maplibre.compose.layers.CircleLayer
import org.maplibre.compose.map.MapOptions
import org.maplibre.compose.map.MaplibreMap
import org.maplibre.compose.map.OrnamentOptions
import org.maplibre.compose.sources.GeoJsonData
import org.maplibre.compose.sources.rememberGeoJsonSource
import org.maplibre.compose.style.BaseStyle
import org.maplibre.compose.util.ClickResult
import org.maplibre.spatialk.geojson.Position

private const val MAP_STYLE = "https://tiles.openfreemap.org/styles/dark"
private const val DEFAULT_ZOOM = 12.0
private const val DEFAULT_LAT = 52.2297
private const val DEFAULT_LNG = 21.0122
private const val TAP_THRESHOLD_DEG = 0.005

@Composable
@Suppress("LongMethod", "CyclomaticComplexMethod", "LongParameterList")
internal fun NearbyEventsContent(
    events: List<Event>,
    selectedEventId: String?,
    userLat: Double?,
    userLng: Double?,
    isPermissionDenied: Boolean,
    onEventSelected: (String) -> Unit,
    onEventClick: (String) -> Unit,
    onRequestPermission: () -> Unit = {},
) {
    if (isPermissionDenied) {
        Column(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Icon(
                PhosphorIcons.Bold.MapPinLine,
                contentDescription = null,
                modifier = Modifier.size(48.dp),
                tint = TextMuted,
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "brak dostępu do lokalizacji",
                fontFamily = NunitoFamily,
                fontSize = 14.sp,
                color = TextMuted,
            )
            Spacer(modifier = Modifier.height(8.dp))
            TextButton(onClick = onRequestPermission) {
                Text(
                    text = "udostępnij lokalizację",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                    color = Primary,
                )
            }
        }
        return
    }

    val effectiveLat = userLat ?: DEFAULT_LAT
    val effectiveLng = userLng ?: DEFAULT_LNG

    val selectedEvent =
        remember(selectedEventId, events) {
            events.find { it.id == selectedEventId }
        }

    val geoEvents =
        remember(events) {
            events.filter { it.latitude != null && it.longitude != null }
        }

    val geocoding = koinInject<GeocodingService>()
    var geocodedLocation by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(selectedEventId) {
        geocodedLocation = null
        val event = events.find { it.id == selectedEventId } ?: return@LaunchedEffect
        val loc = event.location
        if (loc != null && !looksLikeCoordinates(loc)) return@LaunchedEffect
        val lat = event.latitude ?: return@LaunchedEffect
        val lng = event.longitude ?: return@LaunchedEffect
        geocodedLocation = geocoding.reverse(lat, lng)
    }

    Column(modifier = Modifier.fillMaxSize()) {
        // Map container — fill the available height (weight=1f) instead of a fixed 280dp.
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
                    .weight(1f)
                    .clip(RoundedCornerShape(20.dp)),
        ) {
            val cameraState =
                rememberCameraState(
                    firstPosition =
                        CameraPosition(
                            target = Position(latitude = effectiveLat, longitude = effectiveLng),
                            zoom = DEFAULT_ZOOM,
                        ),
                )

            LaunchedEffect(userLat, userLng) {
                if (userLat != null && userLng != null) {
                    cameraState.animateTo(
                        CameraPosition(
                            target = Position(latitude = userLat, longitude = userLng),
                            zoom = DEFAULT_ZOOM,
                        ),
                    )
                }
            }

            val unselectedGeoJson =
                remember(geoEvents, selectedEventId) {
                    multiPointGeoJson(geoEvents.filter { it.id != selectedEventId })
                }

            MaplibreMap(
                modifier = Modifier.fillMaxSize(),
                baseStyle = BaseStyle.Uri(MAP_STYLE),
                cameraState = cameraState,
                options =
                    MapOptions(
                        ornamentOptions =
                            OrnamentOptions(
                                isLogoEnabled = false,
                                isCompassEnabled = false,
                                isScaleBarEnabled = false,
                                isAttributionEnabled = false,
                            ),
                    ),
                onMapClick = { position, _ ->
                    val nearest =
                        geoEvents.minByOrNull {
                            distanceDeg(position.latitude, position.longitude, it.latitude!!, it.longitude!!)
                        }
                    if (nearest != null) {
                        val dist =
                            distanceDeg(
                                position.latitude,
                                position.longitude,
                                nearest.latitude!!,
                                nearest.longitude!!,
                            )
                        if (dist < TAP_THRESHOLD_DEG * TAP_THRESHOLD_DEG) {
                            onEventSelected(nearest.id)
                        }
                    }
                    ClickResult.Consume
                },
            ) {
                // Unselected dots
                if (geoEvents.size > (if (selectedEventId != null) 1 else 0)) {
                    val unselectedSource = rememberGeoJsonSource(data = unselectedGeoJson)
                    CircleLayer(
                        id = "unselected-events",
                        source = unselectedSource,
                        radius = const(7.dp),
                        color = const(Primary),
                        strokeColor = const(Primary),
                        strokeWidth = const(1.dp),
                    )
                }

                // User location dot
                if (userLat != null && userLng != null) {
                    val userSource =
                        rememberGeoJsonSource(
                            data = pointGeoJson(userLat, userLng),
                        )
                    CircleLayer(
                        id = "user-location",
                        source = userSource,
                        radius = const(8.dp),
                        color = const(White),
                        strokeColor = const(Primary),
                        strokeWidth = const(3.dp),
                    )
                }

                // Selected dot
                val selEvent = geoEvents.find { it.id == selectedEventId }
                if (selEvent != null) {
                    val selectedSource =
                        rememberGeoJsonSource(
                            data = pointGeoJson(selEvent.latitude!!, selEvent.longitude!!),
                        )
                    CircleLayer(
                        id = "selected-event",
                        source = selectedSource,
                        radius = const(12.dp),
                        color = const(Primary),
                        strokeColor = const(White),
                        strokeWidth = const(3.dp),
                    )
                }
            }
        }

        // Event info panel
        if (selectedEvent != null) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp))
                        .background(Background)
                        .clickable { onEventClick(selectedEvent.id) },
            ) {
                selectedEvent.coverImage?.let { cover ->
                    AsyncImage(
                        model = resolveImageUrl(cover),
                        contentDescription = null,
                        modifier = Modifier.matchParentSize(),
                        contentScale = ContentScale.Crop,
                    )
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .background(
                                    Brush.verticalGradient(
                                        0.0f to Background.copy(alpha = 0.97f),
                                        1.0f to Background.copy(alpha = 0.88f),
                                    ),
                                ),
                    )
                }
                Column(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 16.dp, vertical = 12.dp)
                            .padding(bottom = LocalNavBarPadding.current),
                ) {
                    Text(
                        text = selectedEvent.title,
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 20.sp,
                        color = TextPrimary,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )

                    Spacer(modifier = Modifier.height(4.dp))

                    Text(
                        text = formatEventDate(selectedEvent.startsAt),
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.Normal,
                        fontSize = 14.sp,
                        color = TextSecondary,
                    )

                    val displayLocation =
                        selectedEvent.location
                            ?.takeIf { !looksLikeCoordinates(it) }
                            ?: geocodedLocation
                    if (displayLocation != null) {
                        Text(
                            text = displayLocation,
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextMuted,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }

                    selectedEvent.description?.let { description ->
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = description,
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 14.sp,
                            color = TextSecondary,
                            maxLines = 3,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }

                    if (selectedEvent.attendeesCount > 0 || selectedEvent.maxAttendees != null) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            if (selectedEvent.attendeesPreview.isNotEmpty()) {
                                StackedAvatars(
                                    imageUrls = selectedEvent.attendeesPreview.map { it.profilePicture },
                                    avatarSize = 28.dp,
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                            }
                            Text(
                                text =
                                    selectedEvent.maxAttendees?.let { "${selectedEvent.attendeesCount} / $it" }
                                        ?: pluralizePolish(
                                            selectedEvent.attendeesCount,
                                            "osoba",
                                            "osoby",
                                            "osób",
                                        ),
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Bold,
                                fontSize = 14.sp,
                                color = TextPrimary,
                            )
                        }
                    }
                }
            }
        } else {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .weight(1f),
                contentAlignment = Alignment.Center,
            ) {
                val hint =
                    if (geoEvents.isEmpty()) {
                        "brak wydarzeń w pobliżu"
                    } else {
                        "wybierz wydarzenie na mapie"
                    }
                Text(
                    text = hint,
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                    color = TextMuted,
                )
            }
        }
    }
}

private fun pointGeoJson(
    lat: Double,
    lng: Double,
): GeoJsonData =
    GeoJsonData.JsonString(
        """{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[$lng,$lat]},"properties":{}}]}""",
    )

private fun multiPointGeoJson(events: List<Event>): GeoJsonData {
    val features =
        events
            .filter { it.latitude != null && it.longitude != null }
            .joinToString(",") { event ->
                """{"type":"Feature","geometry":{"type":"Point","coordinates":[${event.longitude},${event.latitude}]},"properties":{"id":"${event.id}"}}"""
            }
    return GeoJsonData.JsonString(
        """{"type":"FeatureCollection","features":[$features]}""",
    )
}

private fun distanceDeg(
    lat1: Double,
    lng1: Double,
    lat2: Double,
    lng2: Double,
): Double {
    val dLat = lat1 - lat2
    val dLng = lng1 - lng2
    return dLat * dLat + dLng * dLng
}

private fun looksLikeCoordinates(s: String): Boolean = s.matches(Regex("""^-?\d+[.,]\d+\s*,\s*-?\d+[.,]\d+$"""))
