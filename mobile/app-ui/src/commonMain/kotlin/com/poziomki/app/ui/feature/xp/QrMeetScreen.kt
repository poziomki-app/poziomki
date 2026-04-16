package com.poziomki.app.ui.feature.xp

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.CheckCircle
import com.poziomki.app.network.WeatherInfo
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import io.github.alexzhirkevich.qrose.rememberQrCodePainter
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel

@Composable
@Suppress("LongMethod")
fun QrMeetScreen(onBack: () -> Unit) {
    val qrViewModel: QrMeetViewModel = koinViewModel()
    val tasksViewModel: TasksViewModel = koinViewModel()
    val qrState by qrViewModel.state.collectAsState()
    val tasksState by tasksViewModel.state.collectAsState()
    var selectedTab by remember { mutableIntStateOf(0) }

    LaunchedEffect(qrState.scanResult) {
        if (qrState.scanResult != null) {
            qrViewModel.clearScanResult()
        }
    }

    val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        LazyColumn(
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = statusBarTop, bottom = 32.dp),
        ) {
            item {
                ScreenHeader(title = "dzisiaj", onBack = onBack)
                Spacer(Modifier.height(8.dp))
                WeatherCard(weather = tasksState.weather, isLoading = tasksState.isLoadingWeather)
                Spacer(Modifier.height(16.dp))
                SectionLabel("zadania dnia")
                Spacer(Modifier.height(8.dp))
            }

            items(DailyTask.all, key = { it.id }) { task ->
                TaskCard(
                    task = task,
                    isDone = task.id in tasksState.completedTaskIds,
                    isClaiming = tasksState.claimingTaskId == task.id,
                    onClaim = { tasksViewModel.claimTask(task.id) },
                )
                Spacer(Modifier.height(8.dp))
            }

            item {
                Spacer(Modifier.height(8.dp))
                SectionLabel("poznaj kogoś")
                Spacer(Modifier.height(8.dp))
                PrimaryTabRow(selectedTabIndex = selectedTab) {
                    Tab(
                        selected = selectedTab == 0,
                        onClick = { selectedTab = 0 },
                        text = { Text("Mój kod QR") },
                    )
                    Tab(
                        selected = selectedTab == 1,
                        onClick = { selectedTab = 1 },
                        text = { Text("Zeskanuj") },
                    )
                }
                Spacer(Modifier.height(16.dp))
                when (selectedTab) {
                    0 -> {
                        MyQrContent(state = qrState, onRefresh = qrViewModel::loadToken)
                    }

                    1 -> {
                        ScanContent(
                            state = qrState,
                            onScanned = { value ->
                                qrViewModel.onScanInputChange(value)
                                qrViewModel.submitScan()
                            },
                            onRetry = { qrViewModel.onScanInputChange("") },
                        )
                    }
                }
            }
        }

        tasksState.lastXpMessage?.let { msg ->
            Snackbar(
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(16.dp),
            ) { Text(text = msg) }
            LaunchedEffect(msg) {
                delay(2000)
                tasksViewModel.clearXpMessage()
            }
        }
    }
}

@Composable
private fun WeatherCard(
    weather: WeatherInfo?,
    isLoading: Boolean,
) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(PoziomkiTheme.radius.lg))
                .background(SurfaceElevated)
                .padding(PoziomkiTheme.spacing.lg),
    ) {
        when {
            isLoading -> {
                Box(Modifier.fillMaxWidth().height(72.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                }
            }

            weather != null -> {
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(text = weather.emoji, fontSize = 36.sp)
                        Spacer(Modifier.width(PoziomkiTheme.spacing.md))
                        Column {
                            Text(
                                text = "${weather.temperatureC}°C",
                                fontSize = 28.sp,
                                fontWeight = FontWeight.Bold,
                                fontFamily = NunitoFamily,
                                color = TextPrimary,
                            )
                            Text(
                                text = "Warszawa",
                                fontSize = 12.sp,
                                fontFamily = NunitoFamily,
                                color = TextMuted,
                            )
                        }
                    }
                    Spacer(Modifier.height(6.dp))
                    Text(
                        text = "${weather.description} · wiatr ${weather.windSpeedKmh} km/h",
                        fontSize = 13.sp,
                        fontFamily = NunitoFamily,
                        color = TextSecondary,
                    )
                }
            }

            else -> {
                Box(Modifier.fillMaxWidth().height(72.dp), contentAlignment = Alignment.Center) {
                    Text(
                        text = "nie udało się załadować pogody",
                        color = TextMuted,
                        fontFamily = NunitoFamily,
                        fontSize = 13.sp,
                    )
                }
            }
        }
    }
}

@Composable
private fun SectionLabel(text: String) {
    Text(
        text = text.uppercase(),
        fontSize = 11.sp,
        fontWeight = FontWeight.Bold,
        fontFamily = NunitoFamily,
        color = TextMuted,
        letterSpacing = 1.2.sp,
    )
}

@Composable
private fun TaskCard(
    task: DailyTask,
    isDone: Boolean,
    isClaiming: Boolean,
    onClaim: () -> Unit,
) {
    val bgColor by animateColorAsState(
        targetValue = if (isDone) SurfaceElevated.copy(alpha = 0.5f) else SurfaceElevated,
        label = "taskBg",
    )
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(PoziomkiTheme.radius.md))
                .background(bgColor)
                .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.md),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text = task.emoji, fontSize = 22.sp)
        Spacer(Modifier.width(PoziomkiTheme.spacing.md))
        Text(
            text = task.label,
            modifier = Modifier.weight(1f),
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.Medium,
            fontSize = 15.sp,
            color = if (isDone) TextMuted else TextPrimary,
        )
        Spacer(Modifier.width(PoziomkiTheme.spacing.sm))
        if (isDone) {
            Icon(
                imageVector = PhosphorIcons.Bold.CheckCircle,
                contentDescription = null,
                tint = Primary,
                modifier = Modifier.size(22.dp),
            )
        } else {
            FilledTonalButton(
                onClick = onClaim,
                enabled = !isClaiming,
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 0.dp),
                modifier = Modifier.height(34.dp),
            ) {
                if (isClaiming) {
                    CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                } else {
                    Text("+5 XP", fontSize = 13.sp, fontFamily = NunitoFamily, fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }
}

@Composable
private fun MyQrContent(
    state: QrMeetState,
    onRefresh: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        when {
            state.isLoadingToken -> {
                Box(modifier = Modifier.size(240.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }

            state.tokenError != null -> {
                Text(text = state.tokenError, color = TextMuted, textAlign = TextAlign.Center)
                Button(onClick = onRefresh) { Text("Spróbuj ponownie") }
            }

            state.token != null -> {
                val painter = rememberQrCodePainter(data = state.token)
                Image(
                    painter = painter,
                    contentDescription = "Twój kod QR",
                    modifier = Modifier.size(240.dp),
                )
                Text(
                    text = "Pokaż ten kod innej osobie",
                    color = TextMuted,
                    fontSize = 14.sp,
                    textAlign = TextAlign.Center,
                )
                Text(text = "Kod wygasa co 5 minut", color = TextMuted, fontSize = 12.sp)
            }
        }
    }
}

@Composable
private fun ScanContent(
    state: QrMeetState,
    onScanned: (String) -> Unit,
    onRetry: () -> Unit,
) {
    when {
        state.isScanning -> {
            Box(
                modifier = Modifier.fillMaxWidth().height(280.dp),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator()
            }
        }

        state.scanResult != null -> {
            val message =
                when (state.scanResult) {
                    is ScanResult.Awarded -> "+${state.scanResult.xpGained} XP! Miło was poznać 🎉"
                    ScanResult.AlreadyScanned -> "Już dziś zeskanowano ten profil"
                }
            Box(
                modifier = Modifier.fillMaxWidth().height(280.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = message,
                    fontWeight = FontWeight.SemiBold,
                    textAlign = TextAlign.Center,
                    fontSize = 16.sp,
                    color = TextPrimary,
                )
            }
        }

        state.scanError != null -> {
            Column(
                modifier = Modifier.fillMaxWidth().height(280.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                Text(text = state.scanError, color = TextMuted, fontSize = 14.sp, textAlign = TextAlign.Center)
                Spacer(Modifier.height(16.dp))
                Button(onClick = onRetry) { Text("Spróbuj ponownie") }
            }
        }

        else -> {
            CameraQrScanner(
                onScanned = onScanned,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(280.dp)
                        .clip(RoundedCornerShape(PoziomkiTheme.radius.lg)),
            )
        }
    }
}
