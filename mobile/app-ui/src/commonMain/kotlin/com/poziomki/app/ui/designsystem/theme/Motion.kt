package com.poziomki.app.ui.designsystem.theme

import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally

object MotionDurations {
    const val INSTANT = 90
    const val SHORT = 200
    const val MEDIUM = 280
}

// Material 3 "emphasized" easing (https://m3.material.io/styles/motion/easing-and-duration).
// Both screens share this curve so they feel coupled instead of two independent animations.
private val Emphasized = CubicBezierEasing(0.2f, 0.0f, 0.0f, 1.0f)

// Material 3 shared-axis-X pattern, tuned tight. Both incoming and outgoing
// screens travel at the same speed with the same easing, with the outgoing
// fade weighted to the first ~30% so the new screen is fully visible by the
// time it stops moving — that's what makes it feel smooth instead of "two
// things at once".

fun forwardSlide(): EnterTransition =
    slideInHorizontally(
        initialOffsetX = { fullWidth -> (fullWidth * 0.18f).toInt() },
        animationSpec = tween(MotionDurations.MEDIUM, easing = Emphasized),
    ) + fadeIn(tween(durationMillis = MotionDurations.SHORT, delayMillis = 60, easing = Emphasized))

fun forwardSlideExit(): ExitTransition =
    slideOutHorizontally(
        targetOffsetX = { fullWidth -> -(fullWidth * 0.18f).toInt() },
        animationSpec = tween(MotionDurations.MEDIUM, easing = Emphasized),
    ) + fadeOut(tween(MotionDurations.INSTANT, easing = Emphasized))

fun backSlide(): EnterTransition =
    slideInHorizontally(
        initialOffsetX = { fullWidth -> -(fullWidth * 0.18f).toInt() },
        animationSpec = tween(MotionDurations.MEDIUM, easing = Emphasized),
    ) + fadeIn(tween(durationMillis = MotionDurations.SHORT, delayMillis = 60, easing = Emphasized))

fun backSlideExit(): ExitTransition =
    slideOutHorizontally(
        targetOffsetX = { fullWidth -> (fullWidth * 0.18f).toInt() },
        animationSpec = tween(MotionDurations.MEDIUM, easing = Emphasized),
    ) + fadeOut(tween(MotionDurations.INSTANT, easing = Emphasized))

fun tabFadeIn(): EnterTransition = fadeIn(tween(MotionDurations.INSTANT, easing = Emphasized))

fun tabFadeOut(): ExitTransition = fadeOut(tween(MotionDurations.INSTANT, easing = Emphasized))
