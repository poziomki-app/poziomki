package com.poziomki.app.ui.feature.feedback

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.MontserratFamily
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.SurfaceElevated
import com.poziomki.app.ui.designsystem.theme.TextPrimary
import com.poziomki.app.ui.designsystem.theme.TextSecondary
import org.jetbrains.compose.resources.painterResource
import poziomki_mobile.app_ui.generated.resources.Res
import poziomki_mobile.app_ui.generated.resources.doodle_dancing

@Composable
fun WelcomeDialog(onDismiss: () -> Unit) {
    Dialog(onDismissRequest = onDismiss) {
        Surface(shape = RoundedCornerShape(24.dp), color = SurfaceElevated) {
            Column(modifier = Modifier.fillMaxWidth()) {
                WelcomeHero()
                WelcomeBody(onDismiss)
            }
        }
    }
}

@Composable
private fun WelcomeBody(onDismiss: () -> Unit) {
    Column(modifier = Modifier.padding(horizontal = 24.dp, vertical = 20.dp)) {
        Text(
            text = "witaj w poziomkach",
            fontFamily = MontserratFamily,
            fontWeight = FontWeight.ExtraBold,
            fontSize = 22.sp,
            color = TextPrimary,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "studencka apka eventów - to wciąż wczesna wersja, więc coś może działać dziwnie.",
            fontFamily = NunitoFamily,
            fontSize = 14.sp,
            lineHeight = 20.sp,
            color = TextSecondary,
        )
        Spacer(modifier = Modifier.height(16.dp))
        WelcomeBullet("bądź miły dla innych")
        Spacer(modifier = Modifier.height(6.dp))
        WelcomeBullet("to apka eventowa, nie randkowa")
        Spacer(modifier = Modifier.height(6.dp))
        WelcomeBullet("zgłaszaj nadużycia długim przytrzymaniem wiadomości lub z menu profilu.")
        Spacer(modifier = Modifier.height(6.dp))
        WelcomeBullet("napisz co działa, a co nie, w „zostaw opinię”.")
        Spacer(modifier = Modifier.height(24.dp))
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            WelcomePillButton(text = "zaczynamy", onClick = onDismiss)
        }
    }
}

@Composable
private fun WelcomePillButton(
    text: String,
    onClick: () -> Unit,
) {
    val fill = Color(0xFFF2F4F7)
    val contentColor = Color(0xFF0B0F14)
    val rowModifier =
        Modifier
            .clip(RoundedCornerShape(50))
            .background(fill)
            .clickable(onClick = onClick)
            .padding(horizontal = 22.dp, vertical = 10.dp)
    Row(
        modifier = rowModifier,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = text,
            fontFamily = NunitoFamily,
            fontWeight = FontWeight.SemiBold,
            fontSize = 15.sp,
            color = contentColor,
        )
    }
}

@Composable
private fun WelcomeHero() {
    val heroShape = RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp)
    val heroBrush = Brush.verticalGradient(listOf(Primary.copy(alpha = 0.35f), SurfaceElevated))
    val heroModifier =
        Modifier
            .fillMaxWidth()
            .height(140.dp)
            .clip(heroShape)
            .background(heroBrush)
    Box(modifier = heroModifier, contentAlignment = Alignment.Center) {
        Image(
            painter = painterResource(Res.drawable.doodle_dancing),
            contentDescription = null,
            modifier = Modifier.size(140.dp),
        )
    }
}

@Composable
private fun WelcomeBullet(text: String) {
    Row(verticalAlignment = Alignment.Top) {
        Text(text = "•", fontFamily = NunitoFamily, fontSize = 14.sp, color = Primary)
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = text,
            fontFamily = NunitoFamily,
            fontSize = 14.sp,
            lineHeight = 20.sp,
            color = TextSecondary,
        )
    }
}
