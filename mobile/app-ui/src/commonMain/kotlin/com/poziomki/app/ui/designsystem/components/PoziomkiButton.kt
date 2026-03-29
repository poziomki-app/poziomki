package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.poziomki.app.ui.designsystem.theme.Border
import com.poziomki.app.ui.designsystem.theme.Error
import com.poziomki.app.ui.designsystem.theme.NunitoFamily
import com.poziomki.app.ui.designsystem.theme.Primary
import com.poziomki.app.ui.designsystem.theme.White

enum class ButtonVariant { PRIMARY, SECONDARY, OUTLINE, DESTRUCTIVE }

private val DefaultGradient =
    Brush.verticalGradient(listOf(Color(0xFF1A2029), Color(0xFF161B22)))

private val DestructiveGradient =
    Brush.verticalGradient(listOf(Color(0xFF2A1215), Color(0xFF1E0D0F)))

private fun contentColor(variant: ButtonVariant): Color =
    when (variant) {
        ButtonVariant.PRIMARY, ButtonVariant.OUTLINE -> Primary
        ButtonVariant.SECONDARY -> White
        ButtonVariant.DESTRUCTIVE -> Error
    }

private fun borderFor(
    variant: ButtonVariant,
    enabled: Boolean,
): BorderStroke {
    val color =
        if (variant == ButtonVariant.DESTRUCTIVE) {
            if (enabled) Error.copy(alpha = 0.5f) else Error.copy(alpha = 0.2f)
        } else {
            if (enabled) Border else Border.copy(alpha = 0.5f)
        }
    return BorderStroke(1.dp, color)
}

private fun backgroundFor(variant: ButtonVariant): Brush =
    if (variant == ButtonVariant.DESTRUCTIVE) DestructiveGradient else DefaultGradient

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
    val isEnabled = enabled && !loading
    val tint = contentColor(variant).let { if (isEnabled) it else it.copy(alpha = 0.4f) }

    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(28.dp),
        color = Color.Transparent,
        border = borderFor(variant, isEnabled),
    ) {
        Row(
            modifier =
                Modifier
                    .background(backgroundFor(variant))
                    .then(if (isEnabled) Modifier.clickable(onClick = onClick) else Modifier)
                    .padding(horizontal = 20.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (loading) {
                CircularProgressIndicator(
                    modifier = Modifier.size(20.dp),
                    color = tint,
                    strokeWidth = 2.dp,
                )
                if (loadingText != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    ButtonLabel(loadingText, tint)
                }
            } else {
                if (icon != null) {
                    Icon(
                        imageVector = icon,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp),
                        tint = tint,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                }
                ButtonLabel(text, tint)
            }
        }
    }
}

@Composable
private fun ButtonLabel(
    text: String,
    color: Color,
) {
    Text(
        text = text,
        fontFamily = NunitoFamily,
        fontWeight = FontWeight.SemiBold,
        fontSize = 15.sp,
        color = color,
    )
}
