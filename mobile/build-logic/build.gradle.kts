plugins {
    `kotlin-dsl`
}

dependencies {
    implementation(libs.plugins.detekt.get().let { "${it.pluginId}:${it.pluginId}.gradle.plugin:${it.version}" })
    implementation(libs.plugins.ktlint.get().let { "${it.pluginId}:${it.pluginId}.gradle.plugin:${it.version}" })
    implementation(libs.plugins.kotlinMultiplatform.get().let { "${it.pluginId}:${it.pluginId}.gradle.plugin:${it.version}" })
}
