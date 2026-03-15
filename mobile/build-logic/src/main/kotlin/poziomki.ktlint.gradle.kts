import org.jlleitschuh.gradle.ktlint.reporter.ReporterType
import org.jlleitschuh.gradle.ktlint.tasks.BaseKtLintCheckTask
import java.io.File

plugins {
    id("org.jlleitschuh.gradle.ktlint")
}

ktlint {
    version = "1.8.0" // must match libs.versions.toml ktlint version
    ignoreFailures = false
    enableExperimentalRules = true
    verbose = true
    reporters {
        reporter(ReporterType.PLAIN)
        reporter(ReporterType.CHECKSTYLE)
    }
    filter {
        exclude("**/build/**")
    }
}

tasks.withType<BaseKtLintCheckTask>().configureEach {
    val sep = File.separator
    val genSeg = "${sep}generated${sep}"
    val buildSeg = "${sep}build${sep}"
    exclude { it.file.path.contains(genSeg) }
    exclude { it.file.path.contains(buildSeg) }
}

tasks.matching { it.name == "check" }.configureEach {
    dependsOn("ktlintCheck")
}
