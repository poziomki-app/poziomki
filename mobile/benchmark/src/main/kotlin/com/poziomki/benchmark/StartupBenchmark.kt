package com.poziomki.benchmark

import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.StartupTimingMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
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
        }
}
