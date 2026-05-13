package com.poziomki.benchmark

import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.StartupTimingMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class StartupBenchmark {
    @get:Rule
    val rule = MacrobenchmarkRule()

    @Test
    fun startupCompilationNone() = startup(CompilationMode.None())

    @Test
    fun startupCompilationPartial() = startup(CompilationMode.Partial())

    private fun startup(mode: CompilationMode) =
        rule.measureRepeated(
            packageName = "app.poziomki",
            metrics = listOf(StartupTimingMetric(), FrameTimingMetric()),
            iterations = 5,
            startupMode = StartupMode.COLD,
            compilationMode = mode,
        ) {
            pressHome()
            startActivityAndWait()
            // startActivityAndWait() returns at first frame, leaving FrameTimingMetric
            // with too few samples. Wait for first content + a short idle so frame
            // timing covers actual UI (TTID is captured before this, so unaffected).
            device.wait(Until.hasObject(By.pkg("app.poziomki").depth(0)), 5_000)
            device.waitForIdle(1_500)
        }
}
