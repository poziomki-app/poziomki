package com.poziomki.app.ui.feature.auth

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.components.AppButton
import com.poziomki.app.ui.designsystem.components.ButtonVariant
import com.poziomki.app.ui.designsystem.components.PoziomkiLogo
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
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
@OptIn(ExperimentalLayoutApi::class)
@Suppress("LongMethod")
@Composable
fun AuthLandingScreen(
    onSignUpWithEmail: () -> Unit,
    onSignInWithEmail: () -> Unit,
) {
    val backgroundColor = MaterialTheme.colorScheme.background
    var showRegulamin by remember { mutableStateOf(false) }
    var showPolicy by remember { mutableStateOf(false) }

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
                        .alpha(0.92f),
            )
            Box(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                colors =
                                    listOf(
                                        backgroundColor.copy(alpha = 1.0f),
                                        backgroundColor.copy(alpha = 0.92f),
                                        backgroundColor.copy(alpha = 0.50f),
                                        backgroundColor.copy(alpha = 0.98f),
                                    ),
                            ),
                        ),
            )

            Column(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .padding(top = 96.dp, start = 24.dp, end = 24.dp),
            ) {
                PoziomkiLogo(size = 48.dp)
                Spacer(modifier = Modifier.height(4.dp))
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
                    .padding(horizontal = PoziomkiTheme.spacing.lg)
                    .padding(bottom = 96.dp),
            verticalArrangement = Arrangement.Bottom,
        ) {
            AppButton(
                text = "zarejestruj się",
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
                            append("zaloguj się")
                        }
                    },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .clickable { onSignInWithEmail() }
                        .padding(bottom = PoziomkiTheme.spacing.lg),
                textAlign = TextAlign.Center,
            )

            FlowRow(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = "kontynuując, akceptujesz ",
                    fontFamily = NunitoFamily,
                    fontSize = 12.sp,
                    color = TextSecondary,
                )
                Text(
                    text = "regulamin",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 12.sp,
                    color = Primary,
                    modifier = Modifier.clickable { showRegulamin = true },
                )
                Text(
                    text = " i ",
                    fontFamily = NunitoFamily,
                    fontSize = 12.sp,
                    color = TextSecondary,
                )
                Text(
                    text = "politykę prywatności",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 12.sp,
                    color = Primary,
                    modifier = Modifier.clickable { showPolicy = true },
                )
            }
        }
    }

    if (showRegulamin) {
        LegalDocumentDialog(
            title = "regulamin",
            body = regulaminText,
            onDismiss = { showRegulamin = false },
        )
    }

    if (showPolicy) {
        LegalDocumentDialog(
            title = "polityka prywatności",
            body = privacyPolicyText,
            onDismiss = { showPolicy = false },
        )
    }
}
