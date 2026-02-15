package com.poziomki.app.ui.screen.event

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TimePicker
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import coil3.compose.AsyncImage
import com.poziomki.app.ui.component.LocationPickerSheet
import com.poziomki.app.ui.component.pointGeoJson
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.MontserratFamily
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.TextMuted
import com.poziomki.app.ui.theme.TextPrimary
import com.poziomki.app.ui.theme.TextSecondary
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.util.decodeImageBytes
import com.poziomki.app.util.rememberSingleImagePicker
import com.poziomki.app.util.resolveImageUrl
import kotlinx.datetime.Instant
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import org.koin.compose.viewmodel.koinViewModel
import org.maplibre.compose.camera.CameraPosition
import org.maplibre.compose.camera.rememberCameraState
import org.maplibre.compose.expressions.dsl.const
import org.maplibre.compose.layers.CircleLayer
import org.maplibre.compose.map.GestureOptions
import org.maplibre.compose.map.MapOptions
import org.maplibre.compose.map.MaplibreMap
import org.maplibre.compose.map.OrnamentOptions
import org.maplibre.compose.sources.rememberGeoJsonSource
import org.maplibre.compose.style.BaseStyle
import org.maplibre.spatialk.geojson.Position
import com.poziomki.app.ui.theme.Surface as SurfaceColor

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventCreateScreen(
    onBack: () -> Unit,
    onCreated: () -> Unit,
    eventId: String? = null,
) {
    val viewModel = koinViewModel<EventCreateViewModel>()
    val state by viewModel.state.collectAsState()

    val pickImage = rememberSingleImagePicker { bytes ->
        if (bytes != null) viewModel.uploadCoverImage(bytes)
    }

    var showLocationPicker by remember { mutableStateOf(false) }
    var showDatePicker by remember { mutableStateOf(false) }
    var showTimePicker by remember { mutableStateOf(false) }
    var selectedDateMillis by remember { mutableStateOf<Long?>(null) }
    var selectedHour by remember { mutableStateOf(12) }
    var selectedMinute by remember { mutableStateOf(0) }

    val isEditMode = eventId != null

    LaunchedEffect(eventId) {
        if (eventId != null) {
            viewModel.loadEvent(eventId)
        }
    }

    // Parse existing startsAt for display
    val dateDisplay =
        remember(state.startsAt) {
            if (state.startsAt.isNotBlank()) {
                runCatching {
                    val instant = Instant.parse(state.startsAt)
                    val dt = instant.toLocalDateTime(TimeZone.currentSystemDefault())
                    "${dt.dayOfMonth.toString().padStart(2, '0')}.${dt.monthNumber.toString().padStart(2, '0')}.${dt.year}"
                }.getOrDefault("")
            } else {
                ""
            }
        }

    val timeDisplay =
        remember(state.startsAt) {
            if (state.startsAt.isNotBlank()) {
                runCatching {
                    val instant = Instant.parse(state.startsAt)
                    val dt = instant.toLocalDateTime(TimeZone.currentSystemDefault())
                    "${dt.hour.toString().padStart(2, '0')}:${dt.minute.toString().padStart(2, '0')}"
                }.getOrDefault("")
            } else {
                ""
            }
        }

    val topInsets = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()

    val textFieldColors =
        TextFieldDefaults.colors(
            focusedContainerColor = SurfaceColor,
            unfocusedContainerColor = SurfaceColor,
            focusedTextColor = TextPrimary,
            unfocusedTextColor = TextPrimary,
            focusedIndicatorColor = Color.Transparent,
            unfocusedIndicatorColor = Color.Transparent,
            cursorColor = Primary,
        )

    val textFieldStyle =
        TextStyle(
            fontFamily = NunitoFamily,
            fontSize = 16.sp,
            color = TextPrimary,
        )

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Background)
                .padding(top = topInsets),
    ) {
        // Top bar
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 4.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Wstecz",
                    tint = TextPrimary,
                )
            }
            Text(
                text = if (isEditMode) "edytuj wydarzenie" else "nowe wydarzenie",
                style = MaterialTheme.typography.titleMedium,
                color = TextPrimary,
                modifier = Modifier.weight(1f),
            )
        }

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = PoziomkiTheme.spacing.md),
        ) {
            // Cover image section
            SectionLabel("zdjęcie")
            val coverImageUrl = state.coverImageUrl
            val coverImageBytes = state.coverImageBytes

            if (coverImageUrl != null || coverImageBytes != null) {
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(16.dp)),
                ) {
                    if (coverImageUrl != null) {
                        AsyncImage(
                            model = resolveImageUrl(coverImageUrl),
                            contentDescription = "Zdjęcie wydarzenia",
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(1.8f),
                            contentScale = ContentScale.Crop,
                        )
                    } else if (coverImageBytes != null) {
                        val imageBitmap = remember(coverImageBytes) { decodeImageBytes(coverImageBytes) }
                        if (imageBitmap != null) {
                            androidx.compose.foundation.Image(
                                bitmap = imageBitmap,
                                contentDescription = "Zdjęcie wydarzenia",
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .aspectRatio(1.8f),
                                contentScale = ContentScale.Crop,
                            )
                        }
                    }

                    if (state.isUploadingCover) {
                        Box(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(1.8f)
                                    .background(Color.Black.copy(alpha = 0.5f)),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator(color = Primary)
                        }
                    }

                    // Remove button
                    Surface(
                        modifier =
                            Modifier
                                .align(Alignment.TopEnd)
                                .padding(8.dp)
                                .size(32.dp)
                                .clickable { viewModel.removeCoverImage() },
                        shape = CircleShape,
                        color = Color.Black.copy(alpha = 0.6f),
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Icon(
                                imageVector = Icons.Filled.Close,
                                contentDescription = "Usuń zdjęcie",
                                tint = Color.White,
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    }
                }
            } else {
                // Dashed placeholder
                Surface(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .aspectRatio(2.5f)
                            .clickable { pickImage() },
                    shape = RoundedCornerShape(16.dp),
                    color = SurfaceColor,
                    border = BorderStroke(1.5.dp, Border),
                ) {
                    Column(
                        modifier = Modifier.fillMaxSize(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Add,
                            contentDescription = null,
                            tint = Primary,
                            modifier = Modifier.size(32.dp),
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = "dodaj zdjęcie",
                            fontFamily = NunitoFamily,
                            color = Primary,
                            fontSize = 14.sp,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Title
            SectionLabel("nazwa")
            TextField(
                value = state.title,
                onValueChange = viewModel::updateTitle,
                placeholder = {
                    Text(
                        "np. planszówki w akademiku",
                        color = TextMuted,
                        fontFamily = NunitoFamily,
                    )
                },
                colors = textFieldColors,
                textStyle = textFieldStyle,
                shape = RoundedCornerShape(14.dp),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Location
            SectionLabel("lokalizacja")
            Surface(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clickable { showLocationPicker = true },
                shape = RoundedCornerShape(14.dp),
                color = SurfaceColor,
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = state.location.ifBlank { "wybierz lokalizację na mapie" },
                        fontFamily = NunitoFamily,
                        fontSize = 16.sp,
                        color = if (state.location.isBlank()) TextMuted else TextPrimary,
                        maxLines = 2,
                        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f),
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Icon(
                        imageVector = Icons.Filled.LocationOn,
                        contentDescription = null,
                        tint = Primary,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }

            // Small map preview when location is selected
            val lat = state.latitude
            val lng = state.longitude
            if (lat != null && lng != null) {
                Spacer(modifier = Modifier.height(8.dp))
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .height(120.dp)
                            .clip(RoundedCornerShape(14.dp))
                            .clickable { showLocationPicker = true },
                ) {
                    MaplibreMap(
                        modifier = Modifier.fillMaxSize(),
                        baseStyle = BaseStyle.Uri("https://tiles.openfreemap.org/styles/positron"),
                        cameraState =
                            rememberCameraState(
                                firstPosition =
                                    CameraPosition(
                                        target = Position(latitude = lat, longitude = lng),
                                        zoom = 14.0,
                                    ),
                            ),
                        options =
                            MapOptions(
                                gestureOptions = GestureOptions.AllDisabled,
                                ornamentOptions =
                                    OrnamentOptions(
                                        isLogoEnabled = false,
                                        isCompassEnabled = false,
                                        isScaleBarEnabled = false,
                                    ),
                            ),
                    ) {
                        val source =
                            rememberGeoJsonSource(
                                data = pointGeoJson(lat, lng),
                            )
                        CircleLayer(
                            id = "preview-marker",
                            source = source,
                            radius = const(8.dp),
                            color = const(Primary),
                            strokeColor = const(Color.White),
                            strokeWidth = const(2.dp),
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Description
            SectionLabel("opis")
            TextField(
                value = state.description,
                onValueChange = viewModel::updateDescription,
                placeholder = {
                    Text(
                        "co, dla kogo, jak się przygotować",
                        color = TextMuted,
                        fontFamily = NunitoFamily,
                    )
                },
                colors = textFieldColors,
                textStyle = textFieldStyle,
                shape = RoundedCornerShape(14.dp),
                maxLines = 5,
                minLines = 3,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Date and time row
            SectionLabel("start")
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            ) {
                // Date chip
                Surface(
                    modifier =
                        Modifier
                            .weight(2f)
                            .clickable { showDatePicker = true },
                    shape = RoundedCornerShape(14.dp),
                    color = SurfaceColor,
                ) {
                    Text(
                        text = dateDisplay.ifBlank { "01.01.2025" },
                        fontFamily = NunitoFamily,
                        color = if (dateDisplay.isBlank()) TextMuted else TextPrimary,
                        fontSize = 16.sp,
                        modifier = Modifier.padding(16.dp),
                    )
                }

                // Time chip
                Surface(
                    modifier =
                        Modifier
                            .weight(1f)
                            .clickable { showTimePicker = true },
                    shape = RoundedCornerShape(14.dp),
                    color = SurfaceColor,
                ) {
                    Text(
                        text = timeDisplay.ifBlank { "21:00" },
                        fontFamily = NunitoFamily,
                        color = if (timeDisplay.isBlank()) TextMuted else TextPrimary,
                        fontSize = 16.sp,
                        modifier = Modifier.padding(16.dp),
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // End date (optional)
            SectionLabel("koniec (opcjonalnie)")
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            ) {
                Surface(
                    modifier =
                        Modifier
                            .weight(2f)
                            .clickable { /* TODO: end date picker */ },
                    shape = RoundedCornerShape(14.dp),
                    color = SurfaceColor,
                ) {
                    Text(
                        text = "01.01.2025",
                        fontFamily = NunitoFamily,
                        color = TextMuted,
                        fontSize = 16.sp,
                        modifier = Modifier.padding(16.dp),
                    )
                }

                Surface(
                    modifier =
                        Modifier
                            .weight(1f)
                            .clickable { /* TODO: end time picker */ },
                    shape = RoundedCornerShape(14.dp),
                    color = SurfaceColor,
                ) {
                    Text(
                        text = "21:00",
                        fontFamily = NunitoFamily,
                        color = TextMuted,
                        fontSize = 16.sp,
                        modifier = Modifier.padding(16.dp),
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Help text
            Text(
                text = "Po utworzeniu możesz zaprosić ludzi, udostępniając link do czatu wydarzenia.",
                fontFamily = NunitoFamily,
                color = TextMuted,
                fontSize = 13.sp,
            )

            // Error
            state.error?.let { error ->
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                Text(
                    text = error,
                    color = MaterialTheme.colorScheme.error,
                    fontFamily = NunitoFamily,
                    fontSize = 14.sp,
                )
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            // Submit button
            Surface(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(52.dp)
                        .clickable(
                            enabled = !state.isLoading && state.title.isNotBlank() && state.startsAt.isNotBlank(),
                        ) {
                            viewModel.saveEvent(onCreated)
                        },
                shape = RoundedCornerShape(26.dp),
                color =
                    if (state.title.isNotBlank() && state.startsAt.isNotBlank()) {
                        Primary
                    } else {
                        Primary.copy(alpha = 0.3f)
                    },
            ) {
                Box(contentAlignment = Alignment.Center) {
                    if (state.isLoading) {
                        CircularProgressIndicator(
                            color = Background,
                            modifier = Modifier.size(22.dp),
                            strokeWidth = 2.dp,
                        )
                    } else {
                        Text(
                            text = if (isEditMode) "zapisz zmiany" else "utwórz wydarzenie",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.Medium,
                            fontSize = 16.sp,
                            color = Background,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        }
    }

    // Date Picker Dialog
    if (showDatePicker) {
        val datePickerState = rememberDatePickerState(initialSelectedDateMillis = selectedDateMillis)
        DatePickerDialog(
            onDismissRequest = { showDatePicker = false },
            confirmButton = {
                TextButton(onClick = {
                    datePickerState.selectedDateMillis?.let { millis ->
                        selectedDateMillis = millis
                        updateStartsAt(millis, selectedHour, selectedMinute, viewModel)
                    }
                    showDatePicker = false
                }) {
                    Text("OK")
                }
            },
            dismissButton = {
                TextButton(onClick = { showDatePicker = false }) {
                    Text("Anuluj")
                }
            },
        ) {
            DatePicker(state = datePickerState)
        }
    }

    // Location Picker Sheet
    if (showLocationPicker) {
        LocationPickerSheet(
            onDismiss = { showLocationPicker = false },
            onLocationSelected = { name, lat, lng ->
                viewModel.updateLocationWithCoordinates(name, lat, lng)
                showLocationPicker = false
            },
            initialLocation = state.location,
            initialLat = state.latitude,
            initialLng = state.longitude,
        )
    }

    // Time Picker Dialog
    if (showTimePicker) {
        val timePickerState = rememberTimePickerState(initialHour = selectedHour, initialMinute = selectedMinute)
        Dialog(onDismissRequest = { showTimePicker = false }) {
            Surface(
                shape = RoundedCornerShape(28.dp),
                color = MaterialTheme.colorScheme.surface,
            ) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        text = "Wybierz godzinę",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.padding(bottom = 16.dp),
                    )
                    TimePicker(state = timePickerState)
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .padding(top = 16.dp),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        TextButton(onClick = { showTimePicker = false }) {
                            Text("Anuluj")
                        }
                        TextButton(onClick = {
                            selectedHour = timePickerState.hour
                            selectedMinute = timePickerState.minute
                            selectedDateMillis?.let { millis ->
                                updateStartsAt(millis, selectedHour, selectedMinute, viewModel)
                            }
                            showTimePicker = false
                        }) {
                            Text("OK")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SectionLabel(text: String) {
    Text(
        text = text,
        fontFamily = MontserratFamily,
        fontWeight = FontWeight.SemiBold,
        fontSize = 14.sp,
        color = TextPrimary,
        modifier = Modifier.padding(bottom = 8.dp),
    )
}

private fun updateStartsAt(
    dateMillis: Long,
    hour: Int,
    minute: Int,
    viewModel: EventCreateViewModel,
) {
    val dateInstant = Instant.fromEpochMilliseconds(dateMillis)
    val dateLd = dateInstant.toLocalDateTime(TimeZone.currentSystemDefault())
    val isoString =
        "${dateLd.year}-${dateLd.monthNumber.toString().padStart(2, '0')}-${dateLd.dayOfMonth.toString().padStart(2, '0')}T${
            hour.toString().padStart(2, '0')
        }:${minute.toString().padStart(2, '0')}:00Z"
    viewModel.updateStartsAt(isoString)
}
