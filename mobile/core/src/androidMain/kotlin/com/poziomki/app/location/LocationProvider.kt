package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.google.android.gms.tasks.CancellationTokenSource
import kotlinx.coroutines.tasks.await
import kotlinx.coroutines.withTimeoutOrNull

actual class LocationProvider(
    private val context: Context,
) {
    private val client = LocationServices.getFusedLocationProviderClient(context)

    actual fun isPermissionGranted(): Boolean =
        ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_COARSE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED

    @Suppress("MissingPermission")
    actual suspend fun getCurrentLocation(): LocationResult? {
        if (!isPermissionGranted()) return null
        return try {
            // Try last known location first (instant, works without GPS)
            val last = client.lastLocation.await()
            if (last != null) return LocationResult(last.latitude, last.longitude)

            // Fall back to active location request with timeout
            withTimeoutOrNull(LOCATION_TIMEOUT_MS) {
                val cts = CancellationTokenSource()
                val location =
                    client
                        .getCurrentLocation(Priority.PRIORITY_BALANCED_POWER_ACCURACY, cts.token)
                        .await()
                location?.let { LocationResult(it.latitude, it.longitude) }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") _: Exception,
        ) {
            null
        }
    }

    private companion object {
        const val LOCATION_TIMEOUT_MS = 5_000L
    }
}
