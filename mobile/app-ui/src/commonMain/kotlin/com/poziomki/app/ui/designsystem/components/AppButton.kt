package com.poziomki.app.ui.designsystem.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
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
import androidx.compose.runtime.getValue
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

enum class ButtonVariant { PRIMARY, SECONDARY, OUTLINE, DESTRUCTIVE }

private val ButtonShape = RoundedCornerShape(28.dp)

private val DefaultGradient =
    Brush.verticalGradient(listOf(Color(0xFF1A2029), Color(0xFF161B22)))

private val PrimaryGradient =
    Brush.verticalGradient(listOf(Color(0xFF182028), Color(0xFF141A22)))

private val DestructiveGradient =
    Brush.verticalGradient(listOf(Color(0xFF2A1215), Color(0xFF1E0D0F)))

private fun contentColor(variant: ButtonVariant): Color =
    when (variant) {
        ButtonVariant.PRIMARY, ButtonVariant.OUTLINE -> Primary
        ButtonVariant.SECONDARY -> White
        ButtonVariant.DESTRUCTIVE -> Error
    }

private fun backgroundFor(variant: ButtonVariant): Brush =
    when (variant) {
        ButtonVariant.PRIMARY -> PrimaryGradient
        ButtonVariant.DESTRUCTIVE -> DestructiveGradient
        else -> DefaultGradient
    }

@Suppress("LongMethod", "LongParameterList")
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

    val borderModifier = animatedBorder(variant, isEnabled)

    val rowModifier =
        modifier
            .then(borderModifier)
            .clip(ButtonShape)
            .background(backgroundFor(variant))
            .then(if (isEnabled) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 24.dp, vertical = 16.dp)

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

private const val ANIMATION_DURATION = 8000
private const val GLOW_STEPS = 24

@Composable
private fun animatedBorder(
    variant: ButtonVariant,
    enabled: Boolean,
): Modifier {
    if (variant != ButtonVariant.PRIMARY || !enabled) {
        val color =
            when (variant) {
                ButtonVariant.DESTRUCTIVE -> {
                    if (enabled) Error.copy(alpha = 0.5f) else Error.copy(alpha = 0.2f)
                }

                else -> {
                    if (enabled) Border else Border.copy(alpha = 0.5f)
                }
            }
        return Modifier.border(1.dp, color, ButtonShape)
    }

    val spec =
        infiniteRepeatable<Float>(
            animation = tween(ANIMATION_DURATION, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        )
    val transition = rememberInfiniteTransition(label = "border")
    val phase by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = spec,
        label = "borderPhase",
    )

    val brush = glowBrush(phase)
    return Modifier.border(1.dp, brush, ButtonShape)
}

private fun glowBrush(phase: Float): Brush {
    // Generate evenly-spaced stops and compute alpha based on angular distance to peak
    val stops =
        Array(GLOW_STEPS) { i ->
            val pos = i.toFloat() / GLOW_STEPS
            val dist = angularDistance(pos, phase)
            val alpha = glowAlpha(dist)
            pos to Primary.copy(alpha = alpha)
        }
    return Brush.sweepGradient(colorStops = stops)
}

private fun angularDistance(
    a: Float,
    b: Float,
): Float {
    val d = kotlin.math.abs(a - b)
    return kotlin.math.min(d, 1f - d)
}

private fun glowAlpha(dist: Float): Float {
    // Cosine falloff for perfectly smooth gradient, spread ~0.25
    val t = (dist / 0.25f).coerceIn(0f, 1f)
    val smooth = (1f + kotlin.math.cos(t * kotlin.math.PI.toFloat())) / 2f
    return 0.08f + (0.27f * smooth)
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
