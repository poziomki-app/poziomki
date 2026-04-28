package com.poziomki.app.ui.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import kotlinx.coroutines.launch

private const val STATUS_TEXT_MAX = 160

// Curated vibe palette. Twenty entries because that's what fits in 5×4
// without scrolling on a phone, and they cover the most common
// "co teraz robisz" intents. Activity tags will resolve via the
// existing emoji ↔ tag table on the server, so adding/removing here
// doesn't need a backend change.
private val VIBE_EMOJIS =
    listOf(
        "☕",
        "📚",
        "🍕",
        "🏋️",
        "🏃",
        "🚴",
        "🎮",
        "🎬",
        "🎵",
        "🍻",
        "💻",
        "✈️",
        "🌧️",
        "😴",
        "💤",
        "🎲",
        "🤝",
        "🧘",
        "🍳",
        "🚶",
    )

@Suppress("LongParameterList", "LongMethod")
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun StatusComposer(
    currentStatus: String?,
    currentEmoji: String?,
    currentExpiresAt: String?,
    isSaving: Boolean,
    onSave: (emoji: String?, text: String?) -> Unit,
    onClear: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var sheetOpen by remember { mutableStateOf(false) }
    val hasStatus = !currentStatus.isNullOrBlank() || !currentEmoji.isNullOrBlank()

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(horizontal = PoziomkiTheme.spacing.md)
                .background(SurfaceElevated, RoundedCornerShape(50))
                .border(1.dp, Border, RoundedCornerShape(50))
                .clickable { sheetOpen = true }
                .padding(horizontal = 14.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (hasStatus) {
            if (!currentEmoji.isNullOrBlank()) {
                Text(
                    text = currentEmoji,
                    fontSize = 18.sp,
                )
                Spacer(modifier = Modifier.width(8.dp))
            }
            // Emoji-only status: render an empty filler so the expiry label
            // still sits on the right via weight(1f) without leaking the
            // "twój status" placeholder copy alongside the emoji.
            Text(
                text = currentStatus?.takeIf { it.isNotBlank() }.orEmpty(),
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = TextPrimary,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = "wygasa za " + remainingHoursLabel(currentExpiresAt),
                fontFamily = NunitoFamily,
                fontSize = 11.sp,
                color = TextMuted,
            )
        } else {
            Text(
                text = "✨",
                fontSize = 18.sp,
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = "co u ciebie?",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                color = TextMuted,
                modifier = Modifier.weight(1f),
            )
        }
    }

    if (sheetOpen) {
        StatusComposerSheet(
            initialEmoji = currentEmoji,
            initialText = currentStatus.orEmpty(),
            isSaving = isSaving,
            onDismiss = { sheetOpen = false },
            onSave = { emoji, text ->
                onSave(emoji, text)
                sheetOpen = false
            },
            onClear = {
                onClear()
                sheetOpen = false
            },
            hasExistingStatus = hasStatus,
        )
    }
}

@Suppress("LongParameterList", "LongMethod", "CyclomaticComplexMethod")
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun StatusComposerSheet(
    initialEmoji: String?,
    initialText: String,
    isSaving: Boolean,
    hasExistingStatus: Boolean,
    onDismiss: () -> Unit,
    onSave: (String?, String?) -> Unit,
    onClear: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState()
    val scope = rememberCoroutineScope()
    var text by remember { mutableStateOf(initialText) }
    var emoji by remember { mutableStateOf(initialEmoji.orEmpty()) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = SurfaceElevated,
    ) {
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(bottom = PoziomkiTheme.spacing.lg),
            verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.md),
        ) {
            Text(
                text = "co u ciebie?",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 18.sp,
                color = TextPrimary,
            )

            // Emoji row — single line, tap to set. Selected emoji has
            // a filled background; tapping the same one again clears it.
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                VIBE_EMOJIS.forEach { e ->
                    val selected = emoji == e
                    Box(
                        modifier =
                            Modifier
                                .background(
                                    if (selected) Color.White.copy(alpha = 0.12f) else Color.Transparent,
                                    RoundedCornerShape(50),
                                ).border(
                                    1.dp,
                                    if (selected) TextPrimary else Border,
                                    RoundedCornerShape(50),
                                ).clickable { emoji = if (selected) "" else e }
                                .padding(horizontal = 10.dp, vertical = 6.dp),
                    ) {
                        Text(text = e, fontSize = 18.sp)
                    }
                }
            }

            // Text field
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(Color.Black.copy(alpha = 0.18f), RoundedCornerShape(16.dp))
                        .border(1.dp, Border, RoundedCornerShape(16.dp))
                        .padding(horizontal = 14.dp, vertical = 12.dp),
            ) {
                BasicTextField(
                    value = text,
                    onValueChange = { if (it.length <= STATUS_TEXT_MAX) text = it },
                    textStyle =
                        TextStyle(
                            color = TextPrimary,
                            fontFamily = NunitoFamily,
                            fontSize = 15.sp,
                        ),
                    cursorBrush =
                        androidx.compose.ui.graphics
                            .SolidColor(TextPrimary),
                    modifier = Modifier.fillMaxWidth(),
                    decorationBox = { inner ->
                        if (text.isEmpty()) {
                            Text(
                                text = "co teraz robisz?",
                                fontFamily = NunitoFamily,
                                fontSize = 15.sp,
                                color = TextMuted,
                            )
                        }
                        inner()
                    },
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "wygasa za 24h",
                    fontFamily = NunitoFamily,
                    fontSize = 12.sp,
                    color = TextMuted,
                )
                Text(
                    text = "${text.length}/$STATUS_TEXT_MAX",
                    fontFamily = NunitoFamily,
                    fontSize = 12.sp,
                    color = TextMuted,
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (hasExistingStatus) {
                    Box(
                        modifier =
                            Modifier
                                .weight(1f)
                                .height(46.dp)
                                .background(Color.Transparent, RoundedCornerShape(50))
                                .border(1.dp, Border, RoundedCornerShape(50))
                                .clickable(enabled = !isSaving) {
                                    scope.launch { onClear() }
                                },
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = "wyczyść",
                            fontFamily = NunitoFamily,
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 14.sp,
                            color = TextPrimary,
                        )
                    }
                }
                Box(
                    modifier =
                        Modifier
                            .weight(1f)
                            .height(46.dp)
                            .background(TextPrimary, RoundedCornerShape(50))
                            .clickable(enabled = !isSaving && (emoji.isNotBlank() || text.isNotBlank())) {
                                scope.launch {
                                    onSave(
                                        emoji.takeIf { it.isNotBlank() },
                                        text.takeIf { it.isNotBlank() },
                                    )
                                }
                            },
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = if (isSaving) "zapisywanie..." else "zapisz",
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.ExtraBold,
                        fontSize = 14.sp,
                        color = Color.Black,
                    )
                }
            }
        }
    }
}

private fun remainingHoursLabel(expiresAt: String?): String {
    if (expiresAt.isNullOrBlank()) return "—"
    val expiry =
        runCatching { kotlinx.datetime.Instant.parse(expiresAt) }.getOrNull() ?: return "—"
    val now =
        kotlinx.datetime.Clock.System
            .now()
    val remainingMin = (expiry.minus(now)).inWholeMinutes
    return when {
        remainingMin <= 0 -> "0m"
        remainingMin < 60 -> "${remainingMin}m"
        else -> "${remainingMin / 60}h"
    }
}
