plugins {
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.google.services)
    alias(libs.plugins.firebase.perf)
    alias(libs.plugins.firebase.crashlytics)
    id("poziomki.detekt")
    id("poziomki.ktlint")
    id("poziomki.kotlin-warnings")
}

composeCompiler {
    includeTraceMarkers.set(false)
}

// Single source of truth for the app version. release-please bumps this line
// (see .github/.release-please-manifest.json); versionCode is derived so it
// never drifts out of monotonic order.
val appVersionName = "0.22.1" // x-release-please-version

fun computeVersionCode(name: String): Int {
    val parts = name.split(".").map { it.toInt() }
    require(parts.size == 3) { "appVersionName must be major.minor.patch, got '$name'" }
    val (major, minor, patch) = parts
    require(minor in 0..999 && patch in 0..999) {
        "minor/patch must each be < 1000 in the m*1_000_000 + m*1_000 + p scheme (got '$name')"
    }
    return major * 1_000_000 + minor * 1_000 + patch
}

val appVersionCode = computeVersionCode(appVersionName)

android {
    namespace = "com.poziomki.app"
    compileSdk = 36

    buildFeatures {
        buildConfig = true
        compose = true
    }

    defaultConfig {
        applicationId = "app.poziomki"
        minSdk = 24
        targetSdk = 36
        versionCode = appVersionCode
        versionName = appVersionName

        val apiUrl = project.findProperty("apiBaseUrl")?.toString() ?: "http://localhost:5150"
        buildConfigField("String", "API_BASE_URL", "\"$apiUrl\"")
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
        jniLibs {
            useLegacyPackaging = true
        }
    }

    splits {
        abi {
            // AGP 9 refuses APK splits + AAB in the same gradle invocation
            // (issuetracker.google.com/402800800). Release CI calls
            // assembleRelease (splits on) and then bundleRelease with
            // -PnoAbiSplits=true so each task produces a clean output.
            isEnable = !project.hasProperty("noAbiSplits")
            reset()
            include("arm64-v8a", "armeabi-v7a", "x86_64")
            isUniversalApk = true
        }
    }
    buildTypes {
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    lint {
        abortOnError = true
        checkDependencies = true
        warningsAsErrors = true
        // Snapshots pre-existing cosmetic issues (adaptive-icon shape,
        // monochrome layer, locked screen orientation, transitive
        // LogNotTimber). New violations still fail the build.
        baseline = file("lint-baseline.xml")
        // CI lint is one minor version ahead of local and flags 36 as
        // "old"; the release-please bot owns targetSdk bumps anyway.
        // GradleDependency: CI hosts a newer Android SDK (37) than the
        // pinned compileSdk (36) and lint treats that as an error.
        // Same rationale as OldTargetApi — let the bot handle SDK bumps.
        disable += "OldTargetApi"
        disable += "GradleDependency"
    }
}

val releaseStoreFile = providers.gradleProperty("releaseStoreFile").orNull
val releaseStorePassword = providers.gradleProperty("releaseStorePassword").orNull
val releaseKeyAlias = providers.gradleProperty("releaseKeyAlias").orNull
val releaseKeyPassword = providers.gradleProperty("releaseKeyPassword").orNull

if (
    !releaseStoreFile.isNullOrBlank() &&
    !releaseStorePassword.isNullOrBlank() &&
    !releaseKeyAlias.isNullOrBlank() &&
    !releaseKeyPassword.isNullOrBlank()
) {
    android.signingConfigs.create("release") {
        storeFile = file(releaseStoreFile)
        storePassword = releaseStorePassword
        keyAlias = releaseKeyAlias
        keyPassword = releaseKeyPassword
    }
    android.buildTypes.getByName("release").signingConfig = android.signingConfigs.getByName("release")
}

// Refuse to assemble a release with the stub google-services.json. The stub's
// REPLACE_ME api_key produces an APK that throws IllegalArgumentException on
// Firebase init at first launch, and Crashlytics can't report it because it
// depends on the same config. Debug builds keep using the stub freely.
gradle.taskGraph.whenReady {
    val buildingRelease =
        allTasks.any { task ->
            val n = task.name
            (n.startsWith("assemble") || n.startsWith("bundle") || n.startsWith("package")) &&
                n.contains("Release")
        }
    if (!buildingRelease) return@whenReady
    val configFile = file("google-services.json")
    if (!configFile.exists()) {
        throw GradleException("google-services.json is missing; release builds require the real Firebase config")
    }
    val text = configFile.readText()
    if (text.contains("REPLACE_ME") || text.contains("your-firebase-project-id")) {
        throw GradleException(
            "google-services.json looks like the .sample stub; drop the real Firebase config at " +
                "mobile/androidApp/google-services.json before building a release",
        )
    }
}

dependencies {
    implementation(projects.appUi)
    implementation(projects.core)
    implementation(libs.androidx.activity.compose)
    implementation(libs.coil)
    implementation(libs.koin.android)
    implementation(platform(libs.firebase.bom))
    implementation(libs.firebase.messaging)
    implementation(libs.firebase.perf)
    implementation(libs.firebase.crashlytics)
    debugImplementation(libs.androidx.compose.ui.tooling)
}
