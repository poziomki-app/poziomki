import io.gitlab.arturbosch.detekt.extensions.DetektExtension
import java.io.File
import org.jlleitschuh.gradle.ktlint.KtlintExtension
import org.jlleitschuh.gradle.ktlint.reporter.ReporterType
import org.jlleitschuh.gradle.ktlint.tasks.BaseKtLintCheckTask
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.androidLibrary) apply false
    alias(libs.plugins.composeCompiler) apply false
    alias(libs.plugins.composeMultiplatform) apply false
    alias(libs.plugins.detekt) apply false
    alias(libs.plugins.kotlinMultiplatform) apply false
    alias(libs.plugins.ktlint) apply false
    alias(libs.plugins.kotlinxSerialization) apply false
    alias(libs.plugins.sqldelight) apply false
}

val detektVersion = libs.versions.detekt.get()
val ktlintVersion = libs.versions.ktlint.get()

subprojects {
    apply(plugin = "io.gitlab.arturbosch.detekt")
    apply(plugin = "org.jlleitschuh.gradle.ktlint")

    extensions.configure(DetektExtension::class.java) {
        buildUponDefaultConfig = true
        allRules = true
        parallel = true
        autoCorrect = false
        ignoreFailures = false
    }

    dependencies {
        add("detektPlugins", "io.gitlab.arturbosch.detekt:detekt-formatting:$detektVersion")
    }

    extensions.configure(KtlintExtension::class.java) {
        version = ktlintVersion
        ignoreFailures = false
        enableExperimentalRules = true
        verbose = true
        reporters {
            reporter(ReporterType.PLAIN)
            reporter(ReporterType.CHECKSTYLE)
        }
        val generatedPath = "${project.layout.buildDirectory.asFile.get()}${File.separator}generated${File.separator}"
        filter {
            exclude { element -> element.file.path.contains(generatedPath) }
            exclude("**/build/**")
        }
    }

    tasks.withType(KotlinCompile::class.java).configureEach {
        compilerOptions {
            allWarningsAsErrors.set(true)
        }
    }

    val buildGeneratedSegment = "${File.separator}build${File.separator}generated${File.separator}"
    tasks.withType(BaseKtLintCheckTask::class.java).configureEach {
        exclude("**/build/**")
        exclude { element -> element.file.path.contains(buildGeneratedSegment) }
    }

    tasks.matching { it.name == "check" }.configureEach {
        dependsOn("detekt")
        dependsOn("ktlintCheck")
    }
}
