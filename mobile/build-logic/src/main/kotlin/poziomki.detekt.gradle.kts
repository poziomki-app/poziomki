import io.gitlab.arturbosch.detekt.Detekt
import io.gitlab.arturbosch.detekt.DetektCreateBaselineTask
import java.io.File

plugins {
    id("io.gitlab.arturbosch.detekt")
}

detekt {
    buildUponDefaultConfig = true
    allRules = false
    parallel = true
    autoCorrect = false
    ignoreFailures = false
    config.setFrom(rootProject.files("detekt.yml"))
    baseline = file("${project.projectDir}/detekt-baseline.xml")
}

dependencies {
    add("detektPlugins", "io.gitlab.arturbosch.detekt:detekt-formatting:${detekt.toolVersion}")
}

tasks.withType<Detekt>().configureEach {
    val dirs = listOf(
        "src/commonMain/kotlin",
        "src/androidMain/kotlin",
        "src/main/kotlin",
        "src/commonTest/kotlin",
        "src/androidUnitTest/kotlin",
        "src/androidInstrumentedTest/kotlin",
    ).map(::file).filter(File::exists)
    setSource(files(dirs))
    include("**/*.kt", "**/*.kts")
    exclude("**/build/**")
}

tasks.withType<DetektCreateBaselineTask>().configureEach {
    val dirs = listOf(
        "src/commonMain/kotlin",
        "src/androidMain/kotlin",
        "src/main/kotlin",
        "src/commonTest/kotlin",
        "src/androidUnitTest/kotlin",
        "src/androidInstrumentedTest/kotlin",
    ).map(::file).filter(File::exists)
    setSource(files(dirs))
    include("**/*.kt", "**/*.kts")
    exclude("**/build/**")
}

tasks.matching { it.name == "check" }.configureEach {
    dependsOn("detekt")
}
