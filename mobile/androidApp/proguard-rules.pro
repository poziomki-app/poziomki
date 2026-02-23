# kotlinx-serialization — keep @Serializable classes and their generated serializers
-keepattributes *Annotation*, InnerClasses
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

# Ktor — uses reflection and service loading
-keep class io.ktor.** { *; }
-keepclassmembers class io.ktor.** { volatile <fields>; }
-dontwarn io.ktor.**

# OkHttp
-dontwarn okhttp3.internal.platform.**
-dontwarn org.conscrypt.**
-dontwarn org.bouncycastle.**
-dontwarn org.openjsse.**

# Koin
-keep class org.koin.** { *; }

# Coil
-keep class coil3.** { *; }

# SQLDelight — keep library and generated DB classes
-keep class app.cash.sqldelight.** { *; }
-keep class com.poziomki.app.db.** { *; }

# Matrix SDK (Android only, large — keep public API)
-keep class org.matrix.rustcomponents.** { *; }
-keep class uniffi.** { *; }
-dontwarn org.matrix.**

# JNA — required by Matrix SDK Rust FFI (JNI accesses fields/methods by name)
-keep class com.sun.jna.** { *; }
-keep class net.java.dev.jna.** { *; }
-keepclassmembers class com.sun.jna.** { *; }
-keepclassmembers class net.java.dev.jna.** { *; }
-keep class * implements com.sun.jna.Library { *; }
-keep class * implements com.sun.jna.Callback { *; }
-keep class * extends com.sun.jna.Structure { *; }
-keepclassmembers class * extends com.sun.jna.Structure { *; }
-keepclasseswithmembers class * { native <methods>; }
-dontwarn com.sun.jna.**
-dontwarn net.java.dev.jna.**

# uniffi FFI structures (JNA field access by name)
-keep class uniffi.** { *; }
-keepclassmembers class uniffi.** { *; }

# Compose — keep runtime stability
-dontwarn androidx.compose.**

# DataStore
-keep class androidx.datastore.** { *; }

# Keep all app data/API/mapper classes (repositories, models, mappers)
-keep class com.poziomki.app.network.** { *; }
-keep class com.poziomki.app.data.** { *; }
-keep class com.poziomki.app.session.** { *; }
-keep class com.poziomki.app.di.** { *; }

# Keep Kotlin metadata for reflection-based libraries
-keep class kotlin.Metadata { *; }
-dontwarn kotlin.**

# MapLibre
-keep class org.maplibre.** { *; }
-dontwarn org.maplibre.**

# Google Tink / errorprone annotations (used by AndroidX security-crypto)
-dontwarn com.google.errorprone.annotations.CanIgnoreReturnValue
-dontwarn com.google.errorprone.annotations.CheckReturnValue
-dontwarn com.google.errorprone.annotations.Immutable
-dontwarn com.google.errorprone.annotations.RestrictedApi
