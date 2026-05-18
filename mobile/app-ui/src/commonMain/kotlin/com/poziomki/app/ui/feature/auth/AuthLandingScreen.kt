package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.jetbrains.compose.resources.painterResource
import poziomki_mobile.app_ui.generated.resources.Res
import poziomki_mobile.app_ui.generated.resources.login_background

/**
 * Top-level auth landing — the first screen a brand-new user sees.
 *
 * Hero photo + tagline up top, then three entry points: continue with
 * Google (placeholder), sign up with email, sign in with email.
 */
@Suppress("LongMethod")
@Composable
fun AuthLandingScreen(
    onContinueWithGoogle: () -> Unit,
    onSignUpWithEmail: () -> Unit,
    onSignInWithEmail: () -> Unit,
) {
    val backgroundColor = MaterialTheme.colorScheme.background

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(backgroundColor),
    ) {
        // Hero takes the upper ~55 % of the screen — a faded fireplace photo
        // with a vertical gradient scrim that bleeds into the form area.
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .height(540.dp)
                    .clip(RoundedCornerShape(bottomStart = 24.dp, bottomEnd = 24.dp)),
        ) {
            Image(
                painter = painterResource(Res.drawable.login_background),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier =
                    Modifier
                        .fillMaxSize()
                        .alpha(0.75f),
            )
            Box(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                colors =
                                    listOf(
                                        backgroundColor.copy(alpha = 0.10f),
                                        backgroundColor.copy(alpha = 0.30f),
                                        backgroundColor.copy(alpha = 0.95f),
                                    ),
                            ),
                        ),
            )

            Column(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .padding(top = 96.dp, start = 24.dp, end = 24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                PoziomkiLogo(size = 56.dp)
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "poznajmy się!",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Normal,
                    fontSize = 18.sp,
                    color = TextSecondary,
                )
            }
        }

        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(horizontal = PoziomkiTheme.spacing.lg),
            verticalArrangement = Arrangement.Bottom,
        ) {
            Text(
                text = "platforma studentów",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 28.sp,
                color = TextPrimary,
                modifier = Modifier.fillMaxWidth(),
                textAlign = TextAlign.Center,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.xl))

            AppButton(
                text = "kontynuuj z Google",
                onClick = onContinueWithGoogle,
                variant = ButtonVariant.SECONDARY,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                HorizontalDivider(modifier = Modifier.weight(1f), color = TextSecondary.copy(alpha = 0.3f))
                Text(
                    text = "lub",
                    fontFamily = NunitoFamily,
                    fontSize = 13.sp,
                    color = TextSecondary,
                    modifier = Modifier.padding(horizontal = 12.dp),
                )
                HorizontalDivider(modifier = Modifier.weight(1f), color = TextSecondary.copy(alpha = 0.3f))
            }

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            AppButton(
                text = "zarejestruj się e-mailem",
                onClick = onSignUpWithEmail,
                variant = ButtonVariant.PRIMARY,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.lg))

            Text(
                text =
                    buildAnnotatedString {
                        withStyle(
                            SpanStyle(
                                color = Primary,
                                fontFamily = NunitoFamily,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 15.sp,
                                textDecoration = TextDecoration.Underline,
                            ),
                        ) {
                            append("zaloguj się e-mailem")
                        }
                    },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clickable { onSignInWithEmail() }
                        .padding(bottom = PoziomkiTheme.spacing.xl),
                textAlign = TextAlign.Center,
            )
        }
    }
}
