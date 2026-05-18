package com.poziomki.app.ui.feature.feedback

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.Text
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.TextSecondary

@Composable
fun WelcomeDialog(onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                text = "Witaj w Poziomkach!",
                fontFamily = NunitoFamily,
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column {
                WelcomeParagraph(
                    "Dzięki, że pomagasz nam testować aplikację. To jeszcze wersja " +
                        "rozwojowa — niektóre rzeczy mogą działać dziwnie.",
                )
                Spacer(modifier = Modifier.height(12.dp))
                WelcomeBullet("Bądź miły — to społeczność studencka, nie aplikacja randkowa.")
                WelcomeBullet("Nie udostępniaj swoich danych logowania innym.")
                WelcomeBullet("Zgłaszaj nadużycia (długie przytrzymanie wiadomości lub menu profilu).")
                WelcomeBullet("Daj nam znać, co działa, a co nie — kliknij „Zostaw opinię”.")
                Spacer(modifier = Modifier.height(12.dp))
                WelcomeParagraph(
                    "Miłej zabawy i dzięki, że jesteś częścią pierwszego testu.",
                )
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(
                    text = "Rozumiem",
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.Bold,
                )
            }
        },
    )
}

@Composable
private fun WelcomeParagraph(text: String) {
    Text(
        text = text,
        fontFamily = NunitoFamily,
        fontSize = 14.sp,
        color = TextSecondary,
    )
}

@Composable
private fun WelcomeBullet(text: String) {
    Row {
        Text(
            text = "•",
            fontFamily = NunitoFamily,
            fontSize = 14.sp,
            color = TextSecondary,
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = text,
            fontFamily = NunitoFamily,
            fontSize = 14.sp,
            color = TextSecondary,
        )
    }
}
