package com.poziomki.app.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.poziomki.app.api.GeocodingResult
import com.poziomki.app.api.GeocodingService
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
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

private const val DEBOUNCE_MS = 350L
private const val DEFAULT_ZOOM = 13.0

// Warsaw center
private const val DEFAULT_LAT = 52.2297
private const val DEFAULT_LNG = 21.0122

private const val MAP_STYLE = "https://tiles.openfreemap.org/styles/positron"

internal fun pointGeoJson(lat: Double, lng: Double): GeoJsonData =
    GeoJsonData.JsonString(
        """{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[$lng,$lat]},"properties":{}}]}""",
    )

@Composable
fun LocationPickerSheet(
    onDismiss: () -> Unit,
    onLocationSelected: (name: String, lat: Double, lng: Double) -> Unit,
    initialLocation: String = "",
    initialLat: Double? = null,
    initialLng: Double? = null,
) {
    val geocoding = koinInject<GeocodingService>()
    val scope = rememberCoroutineScope()

    var query by remember { mutableStateOf(initialLocation) }
    var results by remember { mutableStateOf<List<GeocodingResult>>(emptyList()) }
    var selectedName by remember { mutableStateOf(initialLocation) }
    var selectedLat by remember { mutableStateOf(initialLat ?: DEFAULT_LAT) }
    var selectedLng by remember { mutableStateOf(initialLng ?: DEFAULT_LNG) }
    var hasSelection by remember { mutableStateOf(initialLat != null) }

    val cameraState =
        rememberCameraState(
            firstPosition =
                CameraPosition(
                    target = Position(latitude = selectedLat, longitude = selectedLng),
                    zoom = DEFAULT_ZOOM,
                ),
        )

    // Debounced search
    LaunchedEffect(query) {
        if (query.length < 3) {
            results = emptyList()
            return@LaunchedEffect
        }
        delay(DEBOUNCE_MS)
        results = geocoding.search(query)
    }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Box(modifier = Modifier.fillMaxSize().background(Background)) {
            // Full-screen map
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
                            ),
                    ),
                onMapClick = { position, _ ->
                    selectedLat = position.latitude
                    selectedLng = position.longitude
                    hasSelection = true
                    results = emptyList()
                    scope.launch {
                        val name = geocoding.reverse(position.latitude, position.longitude)
                        selectedName = name ?: "%.4f, %.4f".format(position.latitude, position.longitude)
                        query = selectedName
                    }
                    ClickResult.Consume
                },
            ) {
                if (hasSelection) {
                    val source = rememberGeoJsonSource(data = pointGeoJson(selectedLat, selectedLng))
                    CircleLayer(
                        id = "selected-marker",
                        source = source,
                        radius = const(10.dp),
                        color = const(Primary),
                        strokeColor = const(Color.White),
                        strokeWidth = const(2.5.dp),
                    )
                }
            }

            val topPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()

            // Floating search + results
            Column(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .align(Alignment.TopCenter)
                        .padding(top = topPadding + 8.dp, start = 16.dp, end = 16.dp),
            ) {
                // Back button
                Surface(
                    modifier = Modifier.size(40.dp),
                    shape = CircleShape,
                    color = Background.copy(alpha = 0.85f),
                ) {
                    IconButton(onClick = onDismiss) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Wstecz",
                            tint = TextPrimary,
                            modifier = Modifier.size(20.dp),
                        )
                    }
                }

                // Search
                TextField(
                    value = query,
                    onValueChange = { query = it },
                    placeholder = {
                        Text("szukaj miejsca...", color = TextMuted, fontFamily = NunitoFamily)
                    },
                    leadingIcon = {
                        Icon(Icons.Filled.Search, contentDescription = null, tint = TextMuted)
                    },
                    colors =
                        TextFieldDefaults.colors(
                            focusedContainerColor = Background.copy(alpha = 0.92f),
                            unfocusedContainerColor = Background.copy(alpha = 0.92f),
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent,
                            cursorColor = Primary,
                        ),
                    textStyle = TextStyle(fontFamily = NunitoFamily, fontSize = 15.sp),
                    shape = RoundedCornerShape(14.dp),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
                )

                // Results
                if (results.isNotEmpty()) {
                    Surface(
                        modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
                        shape = RoundedCornerShape(14.dp),
                        color = Background.copy(alpha = 0.95f),
                    ) {
                        Column(modifier = Modifier.padding(vertical = 4.dp)) {
                            results.forEach { result ->
                                Text(
                                    text = result.name,
                                    fontFamily = NunitoFamily,
                                    fontSize = 14.sp,
                                    color = TextPrimary,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    modifier =
                                        Modifier
                                            .fillMaxWidth()
                                            .clickable {
                                                selectedLat = result.lat
                                                selectedLng = result.lng
                                                selectedName = result.name
                                                hasSelection = true
                                                results = emptyList()
                                                query = result.name
                                                scope.launch {
                                                    cameraState.animateTo(
                                                        CameraPosition(
                                                            target = Position(latitude = result.lat, longitude = result.lng),
                                                            zoom = 15.0,
                                                        ),
                                                    )
                                                }
                                            }
                                            .padding(horizontal = 16.dp, vertical = 12.dp),
                                )
                            }
                        }
                    }
                }
            }

            // Confirm button
            if (hasSelection) {
                Surface(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .align(Alignment.BottomCenter)
                            .padding(horizontal = 16.dp, vertical = 24.dp)
                            .height(52.dp)
                            .clickable { onLocationSelected(selectedName, selectedLat, selectedLng) },
                    shape = RoundedCornerShape(26.dp),
                    color = Primary,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = "wybierz lokalizację",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Medium,
                            fontSize = 16.sp,
                            color = Background,
                        )
                    }
                }
            }
        }
    }
}
