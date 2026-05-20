package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.Background
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.TextPrimary

data class OnboardingPrimaryAction(
    val text: String,
    val onClick: () -> Unit,
    val enabled: Boolean = true,
    val loading: Boolean = false,
    val variant: ButtonVariant = ButtonVariant.PRIMARY,
)

@Composable
@Suppress("LongParameterList", "LongMethod")
fun OnboardingLayout(
    currentStep: Int,
    totalSteps: Int,
    primaryAction: OnboardingPrimaryAction,
    showBack: Boolean = true,
    onBack: (() -> Unit)? = null,
    footerExtras: (@Composable () -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    Scaffold(
        containerColor = Background,
        bottomBar = {
            // IME visibility drives footer presentation: full-width primary
            // pill when the keyboard is closed, and a compact right-aligned
            // pill when it's open (so the action stays reachable without
            // crowding the input). Extra breathing room separates the pill
            // from the keyboard so the user's thumb has space.
            val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
            val navBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
            val imeOpen = imeBottom > navBottom
            val bottomInset =
                if (imeOpen) {
                    imeBottom + PoziomkiTheme.spacing.md
                } else {
                    maxOf(PoziomkiTheme.spacing.xl, navBottom)
                }
            Column(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(Background)
                        .padding(horizontal = PoziomkiTheme.spacing.lg)
                        .padding(top = PoziomkiTheme.spacing.sm)
                        .padding(bottom = bottomInset),
            ) {
                footerExtras?.invoke()
                if (imeOpen) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        OnboardingPillAction(action = primaryAction)
                    }
                } else {
                    AppButton(
                        text = primaryAction.text,
                        onClick = primaryAction.onClick,
                        enabled = primaryAction.enabled,
                        loading = primaryAction.loading,
                        variant = primaryAction.variant,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState()),
        ) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            // Progress bar row — always same height regardless of back button
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(44.dp)
                        .padding(horizontal = PoziomkiTheme.spacing.lg),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (showBack && onBack != null) {
                    IconButton(
                        onClick = onBack,
                        modifier = Modifier.size(36.dp),
                    ) {
                        Icon(
                            imageVector = PhosphorIcons.Bold.ArrowLeft,
                            contentDescription = "Back",
                            tint = TextPrimary,
                        )
                    }
                    Spacer(modifier = Modifier.width(PoziomkiTheme.spacing.sm))
                } else {
                    // Invisible placeholder to keep progress bar aligned
                    Spacer(modifier = Modifier.width(36.dp + PoziomkiTheme.spacing.sm))
                }

                ProgressBar(
                    currentStep = currentStep,
                    totalSteps = totalSteps,
                    modifier = Modifier.weight(1f),
                )

                Spacer(modifier = Modifier.width(36.dp + PoziomkiTheme.spacing.sm))
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            // Content
            content()
        }
    }
}

@Composable
private fun OnboardingPillAction(action: OnboardingPrimaryAction) {
    val isEnabled = action.enabled && !action.loading
    val fill = Color(0xFFF2F4F7)
    val contentColor = Color(0xFF0B0F14).let { if (isEnabled) it else it.copy(alpha = 0.35f) }
    Row(
        modifier =
            Modifier
                .clip(RoundedCornerShape(50))
                .background(fill)
                .then(if (isEnabled) Modifier.clickable(onClick = action.onClick) else Modifier)
                .padding(horizontal = 22.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (action.loading) {
            CircularProgressIndicator(
                modifier = Modifier.size(16.dp),
                color = contentColor,
                strokeWidth = 2.dp,
            )
            Spacer(modifier = Modifier.width(8.dp))
        }
        Text(
            text = action.text,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.SemiBold,
            fontSize = 15.sp,
            color = contentColor,
        )
    }
}

@Composable
fun ProgressBar(
    currentStep: Int,
    totalSteps: Int,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
    ) {
        repeat(totalSteps) { index ->
            val isFilled = index < currentStep
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .height(6.dp)
                        .clip(RoundedCornerShape(50))
                        .background(if (isFilled) Secondary else Border),
            )
        }
    }
}
