package com.poziomki.app.ui.screen.onboarding

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import com.poziomki.app.ui.component.OnboardingLayout
import com.poziomki.app.ui.component.PoziomkiButton
import com.poziomki.app.ui.component.PoziomkiTextField
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun BasicInfoScreen(
    onNext: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()

    OnboardingLayout(
        currentStep = 1,
        totalSteps = 3,
        showBack = false,
        footer = {
            PoziomkiButton(
                text = "dalej",
                onClick = onNext,
                enabled = state.name.isNotBlank() && state.age.toIntOrNull() in 13..150,
            )
        },
    ) {
        Column(
            modifier =
                Modifier
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(bottom = PoziomkiTheme.spacing.lg),
        ) {
            Text(
                text = "podstawowe",
                style = MaterialTheme.typography.headlineMedium,
                color = TextPrimary,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            PoziomkiTextField(
                value = state.name,
                onValueChange = viewModel::updateName,
                label = "jak masz na imi\u0119?",
                placeholder = "imi\u0119",
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions =
                    KeyboardOptions(
                        capitalization = KeyboardCapitalization.Words,
                        imeAction = ImeAction.Next,
                    ),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            PoziomkiTextField(
                value = state.age,
                onValueChange = { if (it.all { c -> c.isDigit() } && it.length <= 3) viewModel.updateAge(it) },
                label = "wiek",
                placeholder = "wiek",
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions =
                    KeyboardOptions(
                        keyboardType = androidx.compose.ui.text.input.KeyboardType.Number,
                        imeAction = ImeAction.Next,
                    ),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            PoziomkiTextField(
                value = state.program,
                onValueChange = viewModel::updateProgram,
                label = "co studiujesz?",
                placeholder = "kierunek",
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions =
                    KeyboardOptions(
                        imeAction = ImeAction.Done,
                    ),
            )
        }
    }
}
