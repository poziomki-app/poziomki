package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Looper
import android.util.Log
import androidx.core.content.ContextCompat
import androidx.core.location.LocationListenerCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.location.LocationRequestCompat
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.location.FusedLocationProviderClient
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

actual class LocationProvider(
    private val context: Context,
) {
    actual fun isPermissionGranted(): Boolean =
        ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_COARSE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED

    actual suspend fun getCurrentLocation(): LocationResult? {
        if (!isPermissionGranted()) {
            Log.w(TAG, "permission not granted")
            return null
        }
        Log.i(TAG, "getCurrentLocation start")
        // 1. Google Fused (FusedLocationProviderClient) — instant on Play Services,
        //    routes through microG/UnifiedNlp on supported AOSP forks.
        val fused = if (isFusedLikelyAvailable()) fusedLocation() else null
        if (fused != null) {
            Log.i(TAG, "fused fix=$fused")
            return fused
        }
        // 2. Raw GPS one-shot — only useful outdoors. Skip if Fused already returned.
        val gps = gpsLocation()
        if (gps != null) {
            Log.i(TAG, "gps fix=$gps")
            return gps
        }
        // 3. ViewModel falls back to Warsaw on null.
        Log.w(TAG, "no fix obtained")
        return null
    }

    // True if the device exposes a Play Services-compatible location backend.
    // Accepts both genuine GMS and microG (where the official check fails without
    // signature spoofing but the package exists and Fused requests are routed
    // through UnifiedNlp).
    private fun isFusedLikelyAvailable(): Boolean {
        val official =
            GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(context) ==
                ConnectionResult.SUCCESS
        if (official) return true
        return runCatching {
            context.packageManager.getPackageInfo(GMS_PACKAGE, 0)
            true
        }.getOrDefault(false)
    }

    @Suppress("MissingPermission", "TooGenericExceptionCaught")
    private suspend fun fusedLocation(): LocationResult? =
        try {
            val client = LocationServices.getFusedLocationProviderClient(context)
            fusedLastLocation(client) ?: fusedCurrentLocation(client)
        } catch (e: Exception) {
            Log.e(TAG, "fused location threw", e)
            null
        }

    @Suppress("MissingPermission")
    private suspend fun fusedLastLocation(client: FusedLocationProviderClient): LocationResult? =
        withTimeoutOrNull(FUSED_LAST_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                client.lastLocation
                    .addOnSuccessListener { location ->
                        if (continuation.isActive) continuation.resume(location?.toLocationResult())
                    }.addOnFailureListener {
                        if (continuation.isActive) continuation.resume(null)
                    }
            }
        }

    @Suppress("MissingPermission")
    private suspend fun fusedCurrentLocation(client: FusedLocationProviderClient): LocationResult? =
        withTimeoutOrNull(FUSED_CURRENT_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                client
                    .getCurrentLocation(Priority.PRIORITY_BALANCED_POWER_ACCURACY, null)
                    .addOnSuccessListener { location ->
                        if (continuation.isActive) continuation.resume(location?.toLocationResult())
                    }.addOnFailureListener { e ->
                        Log.w(TAG, "fused getCurrentLocation failed", e)
                        if (continuation.isActive) continuation.resume(null)
                    }
            }
        }

    @Suppress("MissingPermission")
    private suspend fun gpsLocation(): LocationResult? {
        val manager = context.getSystemService(LocationManager::class.java) ?: return null
        if (!runCatching { manager.isProviderEnabled(LocationManager.GPS_PROVIDER) }.getOrDefault(false)) {
            return null
        }
        return withTimeoutOrNull(GPS_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                val request =
                    LocationRequestCompat
                        .Builder(GPS_TIMEOUT_MS)
                        .setQuality(LocationRequestCompat.QUALITY_BALANCED_POWER_ACCURACY)
                        .setMaxUpdates(1)
                        .setDurationMillis(GPS_TIMEOUT_MS)
                        .build()
                val listener =
                    object : LocationListenerCompat {
                        override fun onLocationChanged(location: Location) {
                            LocationManagerCompat.removeUpdates(manager, this)
                            if (continuation.isActive) continuation.resume(location.toLocationResult())
                        }
                    }
                continuation.invokeOnCancellation {
                    LocationManagerCompat.removeUpdates(manager, listener)
                }
                runCatching {
                    LocationManagerCompat.requestLocationUpdates(
                        manager,
                        LocationManager.GPS_PROVIDER,
                        request,
                        listener,
                        Looper.getMainLooper(),
                    )
                }.onFailure {
                    if (continuation.isActive) continuation.resume(null)
                }
            }
        }
    }

    private companion object {
        const val TAG = "PoziomkiLocation"
        const val GMS_PACKAGE = "com.google.android.gms"
        const val FUSED_LAST_TIMEOUT_MS = 1_500L
        const val FUSED_CURRENT_TIMEOUT_MS = 8_000L
        const val GPS_TIMEOUT_MS = 8_000L
    }
}

private fun Location.toLocationResult(): LocationResult = LocationResult(latitude, longitude)
