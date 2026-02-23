package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.BorderStroke
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
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.Black
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.PoziomkiTheme
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.Secondary
import com.poziomki.app.ui.designsystem.theme.White

enum class ButtonVariant { PRIMARY, SECONDARY, OUTLINE, DESTRUCTIVE }

@Composable
fun PoziomkiButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: ButtonVariant = ButtonVariant.SECONDARY,
    enabled: Boolean = true,
    loading: Boolean = false,
    loadingText: String? = null,
    icon: ImageVector? = null,
) {
    val colors =
        when (variant) {
            ButtonVariant.PRIMARY -> {
                ButtonDefaults.buttonColors(
                    containerColor = Primary,
                    contentColor = Black,
                    disabledContainerColor = Primary.copy(alpha = 0.3f),
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
                    containerColor = Color.Transparent,
                    contentColor = Primary,
                    disabledContainerColor = Color.Transparent,
                    disabledContentColor = Primary.copy(alpha = 0.7f),
                )
            }

            ButtonVariant.DESTRUCTIVE -> {
                ButtonDefaults.buttonColors(
                    containerColor = Error,
                    contentColor = White,
                    disabledContainerColor = Error.copy(alpha = 0.7f),
                    disabledContentColor = White.copy(alpha = 0.7f),
                )
            }
        }

    val border =
        when (variant) {
            ButtonVariant.OUTLINE -> BorderStroke(1.dp, Primary)
            else -> null
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
        border = border,
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
                        fontFamily = NunitoFamily,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 16.sp,
                    )
                }
            }
        } else {
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (icon != null) {
                    Icon(
                        imageVector = icon,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp),
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                }
                Text(
                    text = text,
                    fontFamily = NunitoFamily,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 16.sp,
                )
            }
        }
    }
}
