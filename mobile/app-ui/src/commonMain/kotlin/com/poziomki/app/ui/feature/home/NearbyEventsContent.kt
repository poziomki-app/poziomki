package com.poziomki.app.ui.feature.home

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.rememberInfiniteTransition
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
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
import com.poziomki.app.network.RoutingService
import com.poziomki.app.network.WalkingRoute
import com.poziomki.app.ui.designsystem.Text
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
import org.maplibre.compose.layers.LineLayer
import org.maplibre.compose.map.MapOptions
import org.maplibre.compose.map.MaplibreMap
import org.maplibre.compose.map.OrnamentOptions
import org.maplibre.compose.sources.GeoJsonData
import org.maplibre.compose.sources.rememberGeoJsonSource
import org.maplibre.compose.style.BaseStyle
import org.maplibre.compose.util.ClickResult
import org.maplibre.spatialk.geojson.Position

private const val DEFAULT_ZOOM = 14.0
private const val DEFAULT_LAT = 52.2297
private const val DEFAULT_LNG = 21.0122
private const val TAP_THRESHOLD_DEG = 0.005
private const val USER_DOT_RADIUS_DP = 7f
private const val USER_HALO_MAX_RADIUS_DP = 18f
private const val USER_HALO_PEAK_ALPHA = 0.3f
private const val USER_HALO_CYCLE_MS = 2_200
private val UserHaloEasing = CubicBezierEasing(0.16f, 1f, 0.3f, 1f)

// Warsaw metro bounding box, slightly padded so the city fits with breathing
// room. Pan is clamped to this — outside of Warsaw the nearby tab makes no
// sense yet.
private const val WARSAW_WEST = 20.85
private const val WARSAW_SOUTH = 52.10
private const val WARSAW_EAST = 21.27
private const val WARSAW_NORTH = 52.37

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

    val routing = koinInject<RoutingService>()
    var route by remember { mutableStateOf<WalkingRoute?>(null) }

    LaunchedEffect(selectedEventId) {
        geocodedLocation = null
        val event = events.find { it.id == selectedEventId } ?: return@LaunchedEffect
        val loc = event.location
        if (loc != null && !looksLikeCoordinates(loc)) return@LaunchedEffect
        val lat = event.latitude ?: return@LaunchedEffect
        val lng = event.longitude ?: return@LaunchedEffect
        geocodedLocation = geocoding.reverse(lat, lng)
    }

    // Route is keyed on event + presence of user location only — re-running on
    // every GPS tick would cancel the in-flight request before it returns and
    // the distance would never render.
    val hasUserLoc = userLat != null && userLng != null
    LaunchedEffect(selectedEventId, hasUserLoc) {
        route = null
        if (!hasUserLoc) return@LaunchedEffect
        val event = events.find { it.id == selectedEventId } ?: return@LaunchedEffect
        val evLat = event.latitude ?: return@LaunchedEffect
        val evLng = event.longitude ?: return@LaunchedEffect
        route = routing.walkingRoute(userLat, userLng, evLat, evLng)
    }

    // Layout the map as the full-screen background and float the panel as a
    // bottom overlay. A Column with map weight(1f) + panel below it leaves a
    // small but visible gap above the panel on some devices, presumably from
    // PullToRefreshBox / nested fillMaxSize constraints. Overlaying sidesteps
    // that — the panel sits directly on top of the map's last few rows.
    Box(modifier = Modifier.fillMaxSize()) {
        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp)
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

            // Centre on the user once when their location first resolves. We
            // intentionally do NOT re-key on userLat/userLng — GPS updates every
            // ~second and re-animating would yank the camera away from any pan
            // or zoom the user has made.
            LaunchedEffect(hasUserLoc) {
                if (hasUserLoc) {
                    cameraState.animateTo(
                        CameraPosition(
                            target = Position(latitude = userLat, longitude = userLng),
                            zoom = DEFAULT_ZOOM,
                        ),
                    )
                }
            }

            // Clamp panning to roughly the Warsaw metro area so the user
            // can't drift into open ocean. Passed to MaplibreMap below.
            val warsawBounds =
                remember {
                    org.maplibre.spatialk.geojson.BoundingBox(
                        west = WARSAW_WEST,
                        south = WARSAW_SOUTH,
                        east = WARSAW_EAST,
                        north = WARSAW_NORTH,
                    )
                }

            val unselectedGeoJson =
                remember(geoEvents, selectedEventId) {
                    multiPointGeoJson(geoEvents.filter { it.id != selectedEventId })
                }

            MaplibreMap(
                modifier = Modifier.fillMaxSize(),
                baseStyle = BaseStyle.Json(POZIOMKI_MAP_STYLE_JSON),
                cameraState = cameraState,
                boundingBox = warsawBounds,
                zoomRange = 10f..18f,
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
                // Campus polygons + labels + metro M markers all live in
                // the inline style JSON (PoziomkiMapStyle.kt) — keeps the
                // map self-contained.

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

                // Walking route polyline — drawn underneath the user dot.
                val routeJson = route?.geometryJson
                if (routeJson != null) {
                    val routeSource =
                        rememberGeoJsonSource(data = GeoJsonData.JsonString(featureFromGeometry(routeJson)))
                    LineLayer(
                        id = "user-route",
                        source = routeSource,
                        color = const(Primary),
                        width = const(4.dp),
                        opacity = const(0.85f),
                    )
                }

                // User location: animated halo + solid core dot.
                if (userLat != null && userLng != null) {
                    val userSource =
                        rememberGeoJsonSource(
                            data = pointGeoJson(userLat, userLng),
                        )
                    // One shared cycle drives both radius and opacity. Alpha
                    // starts and ends at 0 so the loop restart is invisible —
                    // no flash, no perceptible redraw seam.
                    val pulse = rememberInfiniteTransition(label = "user-pulse")
                    val haloRadius by pulse.animateFloat(
                        initialValue = USER_DOT_RADIUS_DP,
                        targetValue = USER_DOT_RADIUS_DP,
                        animationSpec =
                            infiniteRepeatable(
                                animation =
                                    keyframes {
                                        durationMillis = USER_HALO_CYCLE_MS
                                        USER_DOT_RADIUS_DP at 0 using UserHaloEasing
                                        USER_HALO_MAX_RADIUS_DP at (USER_HALO_CYCLE_MS - 400)
                                        USER_HALO_MAX_RADIUS_DP at USER_HALO_CYCLE_MS
                                    },
                                repeatMode = RepeatMode.Restart,
                            ),
                        label = "halo-radius",
                    )
                    val haloAlpha by pulse.animateFloat(
                        initialValue = 0f,
                        targetValue = 0f,
                        animationSpec =
                            infiniteRepeatable(
                                animation =
                                    keyframes {
                                        durationMillis = USER_HALO_CYCLE_MS
                                        0f at 0
                                        USER_HALO_PEAK_ALPHA at 180
                                        0f at (USER_HALO_CYCLE_MS - 400)
                                        0f at USER_HALO_CYCLE_MS
                                    },
                                repeatMode = RepeatMode.Restart,
                            ),
                        label = "halo-alpha",
                    )
                    CircleLayer(
                        id = "user-halo",
                        source = userSource,
                        radius = const(haloRadius.dp),
                        color = const(Primary),
                        opacity = const(haloAlpha),
                    )
                    CircleLayer(
                        id = "user-location",
                        source = userSource,
                        radius = const(USER_DOT_RADIUS_DP.dp),
                        color = const(Primary),
                        strokeColor = const(White),
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

        // Event info panel — fixed compact panel below the (much larger) map.
        // No background image, no scroll: title + date + location + attendees,
        // tap → full event screen.
        if (selectedEvent != null) {
            Row(
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .background(Background)
                        .clickable { onEventClick(selectedEvent.id) }
                        .padding(horizontal = 16.dp)
                        .padding(top = 10.dp, bottom = LocalNavBarPadding.current + 20.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val cover = selectedEvent.coverImage
                if (cover != null) {
                    AsyncImage(
                        model = resolveImageUrl(cover),
                        contentDescription = null,
                        contentScale = ContentScale.Crop,
                        modifier =
                            Modifier
                                .size(88.dp)
                                .clip(RoundedCornerShape(14.dp)),
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                }
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = selectedEvent.title,
                        fontFamily = MontserratFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 18.sp,
                        color = TextPrimary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )

                    Spacer(modifier = Modifier.height(2.dp))

                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = formatEventDate(selectedEvent.startsAt),
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Normal,
                            fontSize = 13.sp,
                            color = TextSecondary,
                        )
                        val distanceMeters = route?.distanceMeters
                        if (distanceMeters != null) {
                            Text(
                                text = " · ",
                                fontFamily = NunitoFamily,
                                fontSize = 13.sp,
                                color = TextMuted,
                            )
                            Text(
                                text = formatDistance(distanceMeters),
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Bold,
                                fontSize = 13.sp,
                                color = Primary,
                            )
                        }
                        val displayLocation =
                            selectedEvent.location
                                ?.takeIf { !looksLikeCoordinates(it) }
                                ?: geocodedLocation
                        if (displayLocation != null) {
                            Text(
                                text = " · ",
                                fontFamily = NunitoFamily,
                                fontSize = 13.sp,
                                color = TextMuted,
                            )
                            Text(
                                text = displayLocation,
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.Normal,
                                fontSize = 13.sp,
                                color = TextMuted,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f, fill = false),
                            )
                        }
                    }

                    if (selectedEvent.attendeesCount > 0 || selectedEvent.maxAttendees != null) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            if (selectedEvent.attendeesPreview.isNotEmpty()) {
                                StackedAvatars(
                                    imageUrls = selectedEvent.attendeesPreview.map { it.profilePicture },
                                    avatarSize = 24.dp,
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
                                fontSize = 13.sp,
                                color = TextPrimary,
                            )
                        }
                    }
                }
            }
        }
    }
}

// Wrap a raw GeoJSON geometry (e.g. OSRM LineString) into a FeatureCollection
// so it can be fed to GeoJsonSource.
private fun featureFromGeometry(geometryJson: String): String =
    """{"type":"FeatureCollection","features":[{"type":"Feature","geometry":$geometryJson,"properties":{}}]}"""

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

private fun formatDistance(meters: Double): String =
    if (meters < 1_000) {
        "${meters.toInt()} m"
    } else {
        val km = meters / 1_000.0
        val rounded = (km * 10).toInt() / 10.0
        "$rounded km"
    }
