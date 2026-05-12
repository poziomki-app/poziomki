@file:OptIn(androidx.benchmark.macro.ExperimentalMetricApi::class)

package com.poziomki.benchmark

import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.TraceSectionMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ScreenTraceBenchmark {
    @get:Rule
    val rule = MacrobenchmarkRule()

    // Cold start lands on the AuthGraph's start destination (Route.Login),
    // so the trace section emitted by AppNavigation is "screen:Login".
    @Test
    fun loginScreenTrace() =
        rule.measureRepeated(
            packageName = "app.poziomki",
            metrics = listOf(TraceSectionMetric("screen:Login"), FrameTimingMetric()),
            iterations = 3,
            startupMode = StartupMode.COLD,
        ) {
            startActivityAndWait()
            device.wait(Until.hasObject(By.pkg("app.poziomki").depth(0)), 5_000)
            device.waitForIdle(1_500)
        }
}
