package com.poziomki.app.ui.screen.onboarding

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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.component.OnboardingLayout
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.theme.Border
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.PrimaryLight
import com.poziomki.app.ui.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun InterestsScreen(
    onNext: () -> Unit,
    onBack: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    OnboardingLayout(
        currentStep = 2,
        totalSteps = 3,
        showBack = true,
        onBack = onBack,
        footer = {
            PoziomkiButton(
                text = "dalej",
                onClick = onNext,
                enabled = state.selectedTagIds.size >= 3,
            )
        },
    ) {
        Column(
            modifier =
                Modifier
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(bottom = PoziomkiTheme.spacing.md),
        ) {
            Text(
                text = "zainteresowania",
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
                verticalArrangement = Arrangement.spacedBy(PoziomkiTheme.spacing.sm),
            ) {
                state.availableTags.forEach { tag ->
                    val isSelected = tag.id in state.selectedTagIds
                    TagChip(
                        label = "${tag.emoji ?: ""} ${tag.name}".trim(),
                        selected = isSelected,
                        onClick = { viewModel.toggleTag(tag.id) },
                    )
                }
            }
        }
    }
}

@Composable
private fun TagChip(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val nunito = NunitoFamily
    val shape = RoundedCornerShape(50)
    val bgColor = if (selected) PrimaryLight else androidx.compose.ui.graphics.Color.Transparent
    val borderColor = if (selected) Primary else Border
    val textColor = if (selected) Primary else TextPrimary

    Row(
        modifier =
            modifier
                .clip(shape)
                .background(bgColor, shape)
                .border(1.dp, borderColor, shape)
                .clickable(onClick = onClick)
                .padding(horizontal = 10.dp, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            fontFamily = nunito,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            color = textColor,
        )
    }
}
