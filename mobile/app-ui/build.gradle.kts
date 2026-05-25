import java.io.File

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidKmpLibrary)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.kotlinxSerialization)
    id("poziomki.detekt")
    id("poziomki.ktlint")
    id("poziomki.kotlin-warnings")
}

// See linkerOpts comment below: skiko leaves Skia's hb_* references undefined.
// Find the already-resolved skiko klib on the iOS target's compile classpath
// and return the path where its bundled libharfbuzz.a should be extracted.
// Extraction itself happens via a doFirst hook on the link task so it runs
// after gradle's metadata sync has materialized the klib file on disk.
fun extractedHarfbuzzPath(konanTargetName: String): File =
    layout.buildDirectory
        .file("skiko-harfbuzz/$konanTargetName/libharfbuzz.a")
        .get()
        .asFile

composeCompiler {
    includeTraceMarkers.set(false)
}

kotlin {
    androidLibrary {
        namespace = "com.poziomki.app.ui"
        compileSdk = 36
        minSdk = 24
        androidResources.enable = true
    }

    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64(),
    ).forEach { iosTarget ->
        val konanTargetName = iosTarget.konanTarget.name
        val hbPath = extractedHarfbuzzPath(konanTargetName)
        val compileDeps = iosTarget.compilations.getByName("main").compileDependencyFiles
        val skikoKlib = compileDeps.filter { it.name == "skiko.klib" }
        val extractTask =
            tasks.register<Sync>("extractSkikoHarfbuzz${konanTargetName.replaceFirstChar { it.uppercase() }}") {
                from(zipTree(skikoKlib.singleFile)) {
                    include("default/targets/$konanTargetName/included/libharfbuzz.a")
                    eachFile { path = "libharfbuzz.a" }
                    includeEmptyDirs = false
                }
                into(hbPath.parentFile)
            }
        iosTarget.binaries.framework {
            baseName = "ComposeApp"
            isStatic = true
            // Skiko leaves Skia's hb_* references undefined in its iOS native
            // libs (libharfbuzz.a ships inside the skiko klib but isn't linked
            // into the framework). When the iOS app also links
            // MapLibre.framework — which bundles its own HarfBuzz with an
            // incompatible hb_script_t ABI — ld binds Skia's hb_* references
            // to MapLibre's symbols and the app crashes in hb_shape_full on
            // first text render. Force-loading skiko's libharfbuzz.a here
            // gives Skia its own HarfBuzz back.
            linkerOpts("-force_load", hbPath.absolutePath)
            linkTaskProvider.configure { dependsOn(extractTask) }
        }
    }

    sourceSets {
        androidMain.dependencies {
            implementation(libs.androidx.activity.compose)
            implementation(libs.koin.android)
            implementation(libs.camerax.camera2)
            implementation(libs.camerax.lifecycle)
            implementation(libs.camerax.view)
            implementation(libs.zxing.core)
            implementation(libs.firebase.perf)
        }
        commonMain.dependencies {
            implementation(projects.core)
            implementation(libs.qrose)

            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(compose.components.uiToolingPreview)

            implementation(libs.navigation.compose)
            implementation(libs.lifecycle.runtime.compose)
            implementation(libs.lifecycle.viewmodel.compose)
            implementation(libs.phosphor.icons)
            implementation(libs.coil.compose)
            implementation(libs.coil.network.ktor)
            implementation(libs.koin.core)
            implementation(libs.koin.compose.viewmodel)
            implementation(libs.kotlinx.datetime)
            implementation(libs.maplibre.compose)
        }
    }
}
