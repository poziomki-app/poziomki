package com.poziomki.app.ui.designsystem.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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

enum class ButtonVariant { PRIMARY, SECONDARY, DESTRUCTIVE }

private val ButtonShape = RoundedCornerShape(28.dp)

private val DefaultGradient =
    Brush.verticalGradient(listOf(Color(0xFF1A2029), Color(0xFF161B22)))

private val PrimaryGradient =
    Brush.verticalGradient(listOf(Color(0xFF182028), Color(0xFF141A22)))

private val DestructiveGradient =
    Brush.verticalGradient(listOf(Color(0xFF2A1215), Color(0xFF1E0D0F)))

private fun contentColor(variant: ButtonVariant): Color =
    when (variant) {
        ButtonVariant.PRIMARY -> Primary
        ButtonVariant.SECONDARY -> White
        ButtonVariant.DESTRUCTIVE -> Error
    }

private fun backgroundFor(variant: ButtonVariant): Brush =
    when (variant) {
        ButtonVariant.PRIMARY -> PrimaryGradient
        ButtonVariant.DESTRUCTIVE -> DestructiveGradient
        ButtonVariant.SECONDARY -> DefaultGradient
    }

private fun borderColor(
    variant: ButtonVariant,
    enabled: Boolean,
): Color =
    when (variant) {
        ButtonVariant.DESTRUCTIVE -> if (enabled) Error.copy(alpha = 0.5f) else Error.copy(alpha = 0.2f)
        else -> if (enabled) Border else Border.copy(alpha = 0.5f)
    }

@Suppress("LongParameterList")
@Composable
fun AppButton(
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
    val isIconOnly = text.isEmpty() && icon != null

    val rowModifier =
        modifier
            .border(1.dp, borderColor(variant, isEnabled), ButtonShape)
            .clip(ButtonShape)
            .background(backgroundFor(variant))
            .then(if (isEnabled) Modifier.clickable(onClick = onClick) else Modifier)
            .then(
                if (isIconOnly) {
                    Modifier.padding(16.dp)
                } else {
                    Modifier.padding(horizontal = 28.dp, vertical = 20.dp)
                },
            )

    Row(
        modifier = rowModifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
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
                if (text.isNotEmpty()) Spacer(modifier = Modifier.width(8.dp))
            }
            if (text.isNotEmpty()) ButtonLabel(text, tint)
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
        fontSize = 17.sp,
        color = color,
    )
}
