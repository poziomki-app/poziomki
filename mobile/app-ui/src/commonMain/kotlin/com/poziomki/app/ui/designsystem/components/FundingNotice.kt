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

@Composable
fun FundingNotice(modifier: Modifier = Modifier) {
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(Color.White)
                .padding(16.dp),
    ) {
        Text(
            text =
                "Aplikacja poziomki powstała w ramach projektu „Uczelnie Przyszłości” " +
                    "na Uniwersytecie Warszawskim, dofinansowanego ze środków Unii Europejskiej.",
            preserveCase = true,
            fontFamily = NunitoFamily,
            fontSize = 12.sp,
            color = Color(0xFF242424),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(bottom = 12.dp),
        )
        Image(
            painter = painterResource(Res.drawable.funding_ue),
            contentDescription =
                "Fundusze Europejskie dla Rozwoju Społecznego. Rzeczpospolita Polska. " +
                    "Dofinansowane przez Unię Europejską. Narodowe Centrum Badań i Rozwoju.",
            contentScale = ContentScale.FillWidth,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
