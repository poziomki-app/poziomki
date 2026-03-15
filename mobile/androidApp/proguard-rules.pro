# kotlinx-serialization — keep @Serializable classes and their generated serializers
-keepattributes RuntimeVisibleAnnotations,RuntimeVisibleParameterAnnotations,AnnotationDefault
-dontnote kotlinx.serialization.AnnotationsKt

-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keepclassmembers @kotlinx.serialization.Serializable class com.poziomki.app.** {
    *** Companion;
    *** $serializer;
    <fields>;
}
-keepclasseswithmembers class com.poziomki.app.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.poziomki.app.**$$serializer { *; }

# Ktor — engine/plugin discovery via ServiceLoader + WebSocket internals
-keep class io.ktor.client.engine.okhttp.OkHttpEngineContainer { *; }
-keep class io.ktor.client.plugins.websocket.WebSocketCapability { *; }
-keep class io.ktor.serialization.kotlinx.json.KotlinxSerializationJsonExtensionProvider { *; }
-keep class io.ktor.client.plugins.contentnegotiation.ContentNegotiationCapability { *; }
-keepclassmembers class io.ktor.** { volatile <fields>; }
-dontwarn io.ktor.**

# OkHttp
-dontwarn okhttp3.internal.platform.**
-dontwarn org.conscrypt.**
-dontwarn org.bouncycastle.**
-dontwarn org.openjsse.**

# Compose
-dontwarn androidx.compose.**

# Keep generated SQLDelight DB classes
-keep class com.poziomki.app.db.** { *; }

# Keep Kotlin metadata for reflection-based libraries
-keep class kotlin.Metadata { *; }
-dontwarn kotlin.**

# MapLibre
-dontwarn org.maplibre.**

# Google Tink / errorprone annotations (used by AndroidX security-crypto)
-dontwarn com.google.errorprone.annotations.CanIgnoreReturnValue
-dontwarn com.google.errorprone.annotations.CheckReturnValue
-dontwarn com.google.errorprone.annotations.Immutable
-dontwarn com.google.errorprone.annotations.RestrictedApi

# Repackage classes into unnamed package for smaller DEX (default in AGP 9.1)
-repackageclasses

# Remove Kotlin null-check intrinsics entirely
-processkotlinnullchecks remove
