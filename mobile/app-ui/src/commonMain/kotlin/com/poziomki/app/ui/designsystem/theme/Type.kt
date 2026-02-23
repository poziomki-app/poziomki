package com.poziomki.app.ui.designsystem.theme

import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import org.jetbrains.compose.resources.Font
import poziomki_mobile.app_ui.generated.resources.Res
import poziomki_mobile.app_ui.generated.resources.montserrat_extrabold
import poziomki_mobile.app_ui.generated.resources.nunito_bold
import poziomki_mobile.app_ui.generated.resources.nunito_medium
import poziomki_mobile.app_ui.generated.resources.nunito_regular
import poziomki_mobile.app_ui.generated.resources.nunito_semibold

val MontserratFamily: FontFamily
    @Composable get() =
        FontFamily(
            Font(Res.font.montserrat_extrabold, FontWeight.ExtraBold),
        )

val NunitoFamily: FontFamily
    @Composable get() =
        FontFamily(
            Font(Res.font.nunito_regular, FontWeight.Normal),
            Font(Res.font.nunito_medium, FontWeight.Medium),
            Font(Res.font.nunito_semibold, FontWeight.SemiBold),
            Font(Res.font.nunito_bold, FontWeight.Bold),
        )

@Composable
fun poziomkiTypography(): Typography {
    val montserrat = MontserratFamily
    val nunito = NunitoFamily

    return Typography(
        // Headings — Montserrat ExtraBold
        headlineLarge =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 32.sp,
                lineHeight = 38.sp,
            ),
        headlineMedium =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 28.sp,
                lineHeight = 34.sp,
            ),
        headlineSmall =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 24.sp,
                lineHeight = 30.sp,
            ),
        // Titles — Montserrat ExtraBold (smaller)
        titleLarge =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 20.sp,
                lineHeight = 26.sp,
            ),
        titleMedium =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 18.sp,
                lineHeight = 24.sp,
            ),
        titleSmall =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 16.sp,
                lineHeight = 22.sp,
            ),
        // Body — Nunito
        bodyLarge =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 18.sp,
                lineHeight = 27.sp,
            ),
        bodyMedium =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 16.sp,
                lineHeight = 24.sp,
            ),
        bodySmall =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.Normal,
                fontSize = 14.sp,
                lineHeight = 21.sp,
            ),
        // Labels — Nunito SemiBold
        labelLarge =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
                lineHeight = 24.sp,
            ),
        labelMedium =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 14.sp,
                lineHeight = 21.sp,
            ),
        labelSmall =
            TextStyle(
                fontFamily = nunito,
                fontWeight = FontWeight.Medium,
                fontSize = 12.sp,
                lineHeight = 18.sp,
            ),
        // Display — Montserrat ExtraBold (logo size)
        displayLarge =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 48.sp,
                lineHeight = 56.sp,
            ),
        displayMedium =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 40.sp,
                lineHeight = 48.sp,
            ),
        displaySmall =
            TextStyle(
                fontFamily = montserrat,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 36.sp,
                lineHeight = 44.sp,
            ),
    )
}
