package com.poziomki.app.ui.component

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.theme.Black
import com.poziomki.app.ui.theme.NunitoFamily
import com.poziomki.app.ui.theme.PoziomkiTheme
import com.poziomki.app.ui.theme.Primary
import com.poziomki.app.ui.theme.PrimaryDark
import com.poziomki.app.ui.theme.Secondary
import com.poziomki.app.ui.theme.SecondaryDark

enum class ButtonVariant { PRIMARY, SECONDARY, OUTLINE }

@Composable
fun PoziomkiButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: ButtonVariant = ButtonVariant.SECONDARY,
    enabled: Boolean = true,
    loading: Boolean = false,
    loadingText: String? = null,
) {
    val nunito = NunitoFamily
    val colors =
        when (variant) {
            ButtonVariant.PRIMARY -> {
                ButtonDefaults.buttonColors(
                    containerColor = Primary,
                    contentColor = Black,
                    disabledContainerColor = Primary.copy(alpha = 0.7f),
                    disabledContentColor = Black.copy(alpha = 0.7f),
                )
            }

            ButtonVariant.SECONDARY -> {
                ButtonDefaults.buttonColors(
                    containerColor = Secondary,
                    contentColor = Black,
                    disabledContainerColor = Secondary.copy(alpha = 0.7f),
                    disabledContentColor = Black.copy(alpha = 0.7f),
                )
            }

            ButtonVariant.OUTLINE -> {
                ButtonDefaults.buttonColors(
                    containerColor = androidx.compose.ui.graphics.Color.Transparent,
                    contentColor = Primary,
                    disabledContainerColor = androidx.compose.ui.graphics.Color.Transparent,
                    disabledContentColor = Primary.copy(alpha = 0.7f),
                )
            }
        }

    Button(
        onClick = onClick,
        modifier =
            modifier
                .fillMaxWidth()
                .height(PoziomkiTheme.componentSizes.buttonHeight),
        enabled = enabled && !loading,
        colors = colors,
        shape = RoundedCornerShape(PoziomkiTheme.radius.md),
    ) {
        if (loading) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                CircularProgressIndicator(
                    modifier = Modifier.size(20.dp),
                    color = colors.contentColor,
                    strokeWidth = 2.dp,
                )
                if (loadingText != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = loadingText,
                        fontFamily = nunito,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 16.sp,
                    )
                }
            }
        } else {
            Text(
                text = text,
                fontFamily = nunito,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
            )
        }
    }
}
