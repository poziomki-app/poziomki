@file:OptIn(androidx.benchmark.macro.ExperimentalMetricApi::class)

package com.poziomki.benchmark

import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.TraceSectionMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * End-to-end macrobench: chat list -> open chat room -> scroll.
 *
 * Assumes the device is already logged in (clearPackageData=false in
 * the benchmark module's defaultConfig).
 */
@RunWith(AndroidJUnit4::class)
class ChatFlowBenchmark {
    @get:Rule
    val rule = MacrobenchmarkRule()

    @Test
    fun openChatRoom() =
        rule.measureRepeated(
            packageName = PACKAGE,
            metrics =
                listOf(
                    FrameTimingMetric(),
                    TraceSectionMetric("screen:Messages"),
                    TraceSectionMetric("screen:Chat"),
                ),
            iterations = 5,
            startupMode = StartupMode.WARM,
            compilationMode = CompilationMode.Partial(),
            setupBlock = { pressHome() },
        ) {
            startActivityAndWait()
            device.wait(Until.hasObject(By.pkg(PACKAGE).depth(0)), 5_000)

            // Navigate to Messages tab via the bottom-nav label.
            device.findObject(By.text("Wiadomości"))?.click()
            device.wait(Until.hasObject(By.res(PACKAGE, "chatList")), 5_000)

            // Open the first chat row.
            device.findObject(By.res(PACKAGE, "chatRow"))?.click()
            device.wait(Until.hasObject(By.res(PACKAGE, "chatMessages")), 5_000)
            device.waitForIdle(1_000)

            // Scroll the message list to capture frame timing while jank-prone.
            val messages = device.findObject(By.res(PACKAGE, "chatMessages"))
            messages?.fling(Direction.UP)
            device.waitForIdle(500)
            messages?.fling(Direction.DOWN)
            device.waitForIdle(500)
        }

    companion object {
        private const val PACKAGE = "app.poziomki"
    }
}
