package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Build
import android.os.Looper
import androidx.core.content.ContextCompat
import androidx.core.location.LocationListenerCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.location.LocationRequestCompat
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

actual class LocationProvider(
    private val context: Context,
) {
    private val locationManager = context.getSystemService(LocationManager::class.java)

    actual fun isPermissionGranted(): Boolean =
        ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_COARSE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED

    @Suppress("MissingPermission")
    actual suspend fun getCurrentLocation(): LocationResult? {
        if (!isPermissionGranted() || locationManager == null) return null
        return try {
            lastKnownLocation()?.let { return it.toLocationResult() }

            val provider = activeProvider() ?: return null
            withTimeoutOrNull(LOCATION_TIMEOUT_MS) {
                suspendCancellableCoroutine { continuation ->
                    val request =
                        LocationRequestCompat
                            .Builder(LOCATION_TIMEOUT_MS)
                            .setQuality(LocationRequestCompat.QUALITY_BALANCED_POWER_ACCURACY)
                            .setMaxUpdates(1)
                            .setDurationMillis(LOCATION_TIMEOUT_MS)
                            .build()
                    val listener =
                        object : LocationListenerCompat {
                            override fun onLocationChanged(location: Location) {
                                LocationManagerCompat.removeUpdates(locationManager, this)
                                if (continuation.isActive) {
                                    continuation.resume(location.toLocationResult())
                                }
                            }
                        }
                    continuation.invokeOnCancellation {
                        LocationManagerCompat.removeUpdates(locationManager, listener)
                    }
                    LocationManagerCompat.requestLocationUpdates(
                        locationManager,
                        provider,
                        request,
                        listener,
                        Looper.getMainLooper(),
                    )
                }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") _: Exception,
        ) {
            null
        }
    }

    private fun lastKnownLocation(): Location? =
        LAST_KNOWN_PROVIDERS
            .filter { provider ->
                locationManager
                    ?.let { manager -> runCatching { manager.isProviderEnabled(provider) }.getOrDefault(false) }
                    ?: false
            }.mapNotNull { provider ->
                runCatching { locationManager?.getLastKnownLocation(provider) }.getOrNull()
            }.maxByOrNull(Location::getTime)

    private fun activeProvider(): String? =
        ACTIVE_PROVIDERS.firstOrNull { provider ->
            locationManager
                ?.let { manager -> runCatching { manager.isProviderEnabled(provider) }.getOrDefault(false) }
                ?: false
        }

    private companion object {
        const val LOCATION_TIMEOUT_MS = 15_000L

        // FUSED on Android 12+ is the cheapest and most reliable; otherwise NETWORK
        // (cell+wifi) under COARSE permission. PASSIVE is intentionally excluded —
        // it only forwards fixes other apps already requested, so it cannot
        // deliver an active update on its own.
        val ACTIVE_PROVIDERS: List<String> =
            buildList {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    add(LocationManager.FUSED_PROVIDER)
                }
                add(LocationManager.NETWORK_PROVIDER)
                add(LocationManager.GPS_PROVIDER)
            }

        val LAST_KNOWN_PROVIDERS: List<String> =
            buildList {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    add(LocationManager.FUSED_PROVIDER)
                }
                add(LocationManager.PASSIVE_PROVIDER)
                add(LocationManager.NETWORK_PROVIDER)
                add(LocationManager.GPS_PROVIDER)
            }
    }
}

private fun Location.toLocationResult(): LocationResult = LocationResult(latitude, longitude)
