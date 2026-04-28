package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Build
import android.os.Looper
import android.util.Log
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
        if (!isPermissionGranted()) {
            Log.w(TAG, "permission not granted")
            return null
        }
        if (locationManager == null) {
            Log.w(TAG, "LocationManager unavailable")
            return null
        }
        val enabledProviders = ACTIVE_PROVIDERS.filter { isEnabled(it) }
        Log.i(TAG, "active providers enabled: $enabledProviders")
        lastKnownLocation()?.let {
            Log.i(TAG, "using last-known fix from ${it.provider}")
            return it.toLocationResult()
        }
        val provider = enabledProviders.firstOrNull()
        if (provider == null) {
            Log.w(TAG, "no usable provider; aborting active fix")
            return null
        }
        Log.i(TAG, "requesting active fix from $provider (timeout ${LOCATION_TIMEOUT_MS}ms)")
        return try {
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
            }.also { result ->
                if (result == null) Log.w(TAG, "active fix from $provider timed out")
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Log.w(TAG, "active fix from $provider threw", e)
            null
        }
    }

    private fun isEnabled(provider: String): Boolean =
        locationManager?.let { runCatching { it.isProviderEnabled(provider) }.getOrDefault(false) } ?: false

    private fun lastKnownLocation(): Location? =
        LAST_KNOWN_PROVIDERS
            .filter(::isEnabled)
            .mapNotNull { runCatching { locationManager?.getLastKnownLocation(it) }.getOrNull() }
            .maxByOrNull(Location::getTime)

    private companion object {
        const val TAG = "LocationProvider"
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
