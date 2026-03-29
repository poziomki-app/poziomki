package com.poziomki.app.ui.feature.onboarding

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.X
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.OnboardingLayout
import com.poziomki.app.ui.designsystem.components.PoziomkiTextField
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun BasicInfoScreen(
    onNext: () -> Unit,
    onBack: () -> Unit,
    viewModel: OnboardingViewModel = koinViewModel(),
) {
    val state by viewModel.state.collectAsState()
    var showDegreeSuggestions by remember { mutableStateOf(false) }

    OnboardingLayout(
        currentStep = 1,
        totalSteps = 3,
        showBack = true,
        onBack = onBack,
        footer = {
            AppButton(
                text = "dalej",
                onClick = onNext,
                enabled = state.name.isNotBlank(),
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
                value = state.program,
                onValueChange = { value ->
                    viewModel.updateProgram(value)
                    showDegreeSuggestions = value.isNotEmpty()
                },
                label = "co studiujesz?",
                placeholder = "kierunek",
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions =
                    KeyboardOptions(
                        imeAction = ImeAction.Done,
                    ),
                trailingContent =
                    if (state.program.isNotEmpty()) {
                        {
                            IconButton(
                                onClick = {
                                    viewModel.updateProgram("")
                                    showDegreeSuggestions = false
                                },
                                modifier = Modifier.size(40.dp),
                            ) {
                                Icon(
                                    imageVector = PhosphorIcons.Bold.X,
                                    contentDescription = "wyczyść",
                                    tint = TextMuted,
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                        }
                    } else {
                        null
                    },
            )

            if (showDegreeSuggestions && state.degreeSearchResults.isNotEmpty()) {
                Surface(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(top = 4.dp),
                    shape = RoundedCornerShape(12.dp),
                    color = SurfaceElevated,
                    shadowElevation = 4.dp,
                ) {
                    Column(modifier = Modifier.padding(vertical = 4.dp)) {
                        state.degreeSearchResults.forEach { degree ->
                            Text(
                                text = highlightMatch(degree.name, state.program),
                                fontFamily = NunitoFamily,
                                fontSize = 14.sp,
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .clickable {
                                            viewModel.updateProgram(degree.name)
                                            showDegreeSuggestions = false
                                        }.padding(horizontal = 16.dp, vertical = 10.dp),
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun highlightMatch(
    text: String,
    query: String,
) = buildAnnotatedString {
    if (query.isBlank()) {
        withStyle(SpanStyle(color = TextPrimary)) { append(text) }
        return@buildAnnotatedString
    }
    val lowerText = text.lowercase()
    val lowerQuery = query.lowercase()
    var current = 0
    while (current < text.length) {
        val matchIndex = lowerText.indexOf(lowerQuery, current)
        if (matchIndex == -1) {
            withStyle(SpanStyle(color = TextPrimary)) { append(text.substring(current)) }
            break
        }
        if (matchIndex > current) {
            withStyle(SpanStyle(color = TextPrimary)) { append(text.substring(current, matchIndex)) }
        }
        withStyle(SpanStyle(color = Primary)) {
            append(text.substring(matchIndex, matchIndex + query.length))
        }
        current = matchIndex + query.length
    }
}
