package com.poziomki.app.ui.feature.event

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Bold
import com.adamglin.phosphoricons.bold.ArrowLeft
import com.poziomki.app.network.Event
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextMuted
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
fun EventChatLoadingView(onBack: () -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding(),
    ) {
        IconButton(
            onClick = onBack,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.ArrowLeft,
                contentDescription = "Wstecz",
                tint = TextPrimary,
            )
        }
        CircularProgressIndicator(color = Primary, modifier = Modifier.align(Alignment.Center))
    }
}

@Composable
fun EventChatNotFoundView(onBack: () -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding(),
    ) {
        IconButton(
            onClick = onBack,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            Icon(
                imageVector = PhosphorIcons.Bold.ArrowLeft,
                contentDescription = "Wstecz",
                tint = TextPrimary,
            )
        }
        Text("Nie znaleziono wydarzenia", color = TextSecondary, modifier = Modifier.align(Alignment.Center))
    }
}

@Composable
@Suppress("LongMethod")
fun EventChatJoinRequiredView(
    event: Event,
    isUpdatingAttendance: Boolean,
    onJoin: () -> Unit,
    onBack: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState()),
    ) {
        EventCoverImage(event = event) {
            Surface(
                modifier =
                    Modifier
                        .align(Alignment.TopStart)
                        .statusBarsPadding()
                        .padding(horizontal = 8.dp, vertical = 8.dp)
                        .size(40.dp),
                shape = CircleShape,
                color = Color.Black.copy(alpha = 0.45f),
            ) {
                IconButton(onClick = onBack) {
                    Icon(
                        imageVector = PhosphorIcons.Bold.ArrowLeft,
                        contentDescription = "Wstecz",
                        tint = Color.White,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }

            Column(
                modifier =
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(horizontal = PoziomkiTheme.spacing.md, vertical = PoziomkiTheme.spacing.sm),
            ) {
                Text(
                    text = event.title,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.ExtraBold,
                    color = Color.White,
                )
            }
        }

        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = PoziomkiTheme.spacing.md),
        ) {
            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))

            EventMetaRows(event = event)

            if (event.requiresApproval) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "wymaga akceptacji organizatora",
                    fontFamily = MontserratFamily,
                    fontSize = 13.sp,
                    color = TextMuted,
                )
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            if (event.isPending) {
                AppButton(
                    text = "oczekuje na akceptację",
                    onClick = {},
                    variant = ButtonVariant.SECONDARY,
                    enabled = false,
                )
            } else {
                AppButton(
                    text = "dołącz",
                    onClick = onJoin,
                    variant = ButtonVariant.PRIMARY,
                    loading = isUpdatingAttendance,
                )
            }

            event.description?.let { description ->
                if (description.isNotBlank()) {
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))
                    Text(
                        text = "o wydarzeniu",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary,
                    )
                    Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xs))
                    Text(
                        text = description,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                    )
                }
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))
        }
    }
}
