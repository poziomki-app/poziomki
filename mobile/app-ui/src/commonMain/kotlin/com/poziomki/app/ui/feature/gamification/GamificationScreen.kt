package com.poziomki.app.ui.feature.gamification

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Snackbar
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.Fill
import com.adamglin.phosphoricons.bold.Info
import com.adamglin.phosphoricons.bold.QrCode
import com.adamglin.phosphoricons.bold.Scan
import com.adamglin.phosphoricons.fill.Flame
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ScreenHeader
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.Surface
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import com.poziomki.app.ui.shared.rememberCodeScanner
import io.github.alexzhirkevich.qrose.rememberQrCodePainter
import kotlinx.coroutines.delay
import org.koin.compose.viewmodel.koinViewModel
import kotlin.math.max

@Suppress("LongMethod")
@Composable
fun GamificationScreen(
    onBack: () -> Unit,
    viewModel: GamificationViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    val launchScanner = rememberCodeScanner { token -> viewModel.onScanResult(token) }
    val statusBarTop = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val navBarBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    var showInfo by remember { mutableStateOf(false) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background),
    ) {
        ScreenHeader(
            title = "streak",
            onBack = onBack,
            modifier = Modifier.padding(top = statusBarTop),
            actions = {
                IconButton(onClick = { showInfo = true }) {
                    Icon(
                        PhosphorIcons.Bold.Info,
                        contentDescription = "Jak działa XP",
                        tint = TextPrimary,
                        modifier = Modifier.size(22.dp),
                    )
                }
            },
        )
        if (showInfo) {
            XpInfoDialog(onDismiss = { showInfo = false })
        }

        Box(modifier = Modifier.fillMaxSize()) {
            Column(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .verticalScroll(rememberScrollState())
                        .padding(horizontal = PoziomkiTheme.spacing.lg),
            ) {
                StreakHero(streak = state.streakCurrent, xp = state.xp)
                Spacer(Modifier.height(PoziomkiTheme.spacing.lg))

                SectionTitle("Spotkaj kogoś na żywo")
                Spacer(Modifier.height(8.dp))

                MyQrCard(token = state.myToken, loading = state.isLoadingToken)
                Spacer(Modifier.height(12.dp))

                AppButton(
                    text = "zeskanuj QR znajomego",
                    onClick = launchScanner,
                    icon = PhosphorIcons.Bold.Scan,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(24.dp + navBarBottom))
            }

            val scanXp = state.lastScanXp
            val err = state.errorMessage
            val snack =
                when {
                    scanXp != null && scanXp > 0 -> "+$scanXp XP! Miłego spotkania"
                    scanXp == 0 -> "Już skanowałeś tę osobę dzisiaj."
                    err != null -> err
                    else -> null
                }
            if (snack != null) {
                Snackbar(
                    modifier =
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(PoziomkiTheme.spacing.md),
                ) {
                    Text(snack)
                }
                LaunchedEffect(snack) {
                    delay(2500)
                    viewModel.clearMessage()
                }
            }
        }
    }
}

@Composable
private fun StreakHero(
    streak: Int,
    xp: Int,
) {
    val display = max(1, streak)
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(20.dp))
                .background(Surface)
                .border(1.dp, Border, RoundedCornerShape(20.dp))
                .padding(vertical = 24.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                PhosphorIcons.Fill.Flame,
                contentDescription = null,
                tint = Secondary,
                modifier = Modifier.size(64.dp),
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = display.toString(),
                color = TextPrimary,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 56.sp,
            )
            Text(
                text = if (display == 1) "day streak!" else "dni z rzędu!",
                color = Secondary,
                fontWeight = FontWeight.Bold,
                fontSize = 16.sp,
            )
            Spacer(Modifier.height(12.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = "XP",
                    color = TextMuted,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                )
                Spacer(Modifier.width(6.dp))
                Text(
                    text = xp.toString(),
                    color = Primary,
                    fontSize = 20.sp,
                    fontWeight = FontWeight.ExtraBold,
                )
            }
        }
    }
}

@Composable
private fun XpInfoDialog(onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text("rozumiem", fontWeight = FontWeight.Bold)
            }
        },
        title = {
            Text(
                "Jak zdobywać XP?",
                fontWeight = FontWeight.ExtraBold,
                fontSize = 20.sp,
            )
        },
        text = {
            Column {
                XpInfoRow("+5 XP", "za otwarcie aplikacji dzisiaj")
                Spacer(Modifier.height(8.dp))
                XpInfoRow("+5 XP", "za wysłanie wiadomości")
                Spacer(Modifier.height(8.dp))
                XpInfoRow("+5 XP", "za otwarcie wydarzenia")
                Spacer(Modifier.height(8.dp))
                XpInfoRow("+25 XP", "za spotkanie na żywo — zeskanuj QR znajomego")
                Spacer(Modifier.height(12.dp))
                Text(
                    "Streak rośnie o 1 każdego dnia, gdy zdobędziesz choć 1 XP. Pomijasz dzień → streak wraca do 1.",
                    color = TextSecondary,
                    fontSize = 13.sp,
                )
            }
        },
    )
}

@Composable
private fun XpInfoRow(
    amount: String,
    label: String,
) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(
            amount,
            color = Secondary,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 14.sp,
            modifier = Modifier.width(64.dp),
        )
        Text(label, color = TextPrimary, fontSize = 14.sp)
    }
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text = text,
        color = TextPrimary,
        fontWeight = FontWeight.ExtraBold,
        fontSize = 18.sp,
        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Suppress("LongMethod")
@Composable
private fun MyQrCard(
    token: String?,
    loading: Boolean,
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(16.dp))
                .background(Surface)
                .border(1.dp, Border, RoundedCornerShape(16.dp))
                .padding(16.dp),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Icon(
                PhosphorIcons.Bold.QrCode,
                contentDescription = null,
                tint = Primary,
                modifier = Modifier.size(20.dp),
            )
            Spacer(Modifier.width(8.dp))
            Text(
                "Twój kod do skanowania",
                color = TextPrimary,
                fontWeight = FontWeight.Bold,
                fontSize = 14.sp,
            )
        }
        Spacer(Modifier.height(12.dp))
        Box(
            modifier =
                Modifier
                    .size(220.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(Color.White),
            contentAlignment = Alignment.Center,
        ) {
            when {
                loading -> {
                    CircularProgressIndicator(color = Primary, strokeWidth = 2.dp)
                }

                token != null -> {
                    Image(
                        painter = rememberQrCodePainter(data = token),
                        contentDescription = "Mój kod QR",
                        modifier = Modifier.size(200.dp),
                    )
                }

                else -> {
                    Text(
                        "Brak kodu",
                        color = Color.Gray,
                    )
                }
            }
        }
        Spacer(Modifier.height(8.dp))
        Text(
            text = "Pokaż znajomemu, oboje dostaniecie +25 XP.",
            color = TextSecondary,
            fontSize = 12.sp,
        )
    }
}
