package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import org.jetbrains.compose.resources.painterResource
import poziomki_mobile.app_ui.generated.resources.Res
import poziomki_mobile.app_ui.generated.resources.funding_ue

private const val FUNDING_INTRO = "Aplikacja poziomki powstała w ramach projektu „Uczelnie Przyszłości” "
private const val FUNDING_UNIVERSITY = "na Uniwersytecie Warszawskim, dofinansowanego ze środków Unii Europejskiej."
private const val FUNDING_TEXT = FUNDING_INTRO + FUNDING_UNIVERSITY
private const val FUNDING_INSTITUTIONS = "Fundusze Europejskie, Rzeczpospolita Polska, Unia Europejska, "
private const val FUNDING_DESCRIPTION = FUNDING_INSTITUTIONS + "Uczelnie Przyszłości, NCBR."

@Composable
fun FundingNotice(modifier: Modifier = Modifier) {
    val fundingModifier = modifier.fillMaxWidth().background(Color.White).padding(16.dp)
    Column(modifier = fundingModifier) {
        Text(
            text = FUNDING_TEXT,
            preserveCase = true,
            fontFamily = NunitoFamily,
            fontSize = 12.sp,
            color = Color(0xFF242424),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(bottom = 12.dp),
        )
        Image(
            painter = painterResource(Res.drawable.funding_ue),
            contentDescription = FUNDING_DESCRIPTION,
            contentScale = ContentScale.FillWidth,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
