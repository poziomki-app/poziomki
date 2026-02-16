package com.poziomki.app.ui.component

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.poziomki.app.ui.theme.Background
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Secondary
import com.poziomki.app.ui.theme.TextPrimary

@Composable
fun OnboardingLayout(
    currentStep: Int,
    totalSteps: Int,
    showBack: Boolean = true,
    onBack: (() -> Unit)? = null,
    footer: @Composable () -> Unit,
    content: @Composable () -> Unit,
) {
    Scaffold(
        containerColor = Background,
        bottomBar = {
            Column(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(Background)
                        .padding(horizontal = PoziomkiTheme.spacing.lg)
                        .padding(top = PoziomkiTheme.spacing.sm)
                        .padding(
                            bottom =
                                maxOf(
                                    PoziomkiTheme.spacing.xl,
                                    WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding(),
                                ),
                        ),
            ) {
                footer()
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
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
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
