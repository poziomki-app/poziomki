package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.google.android.gms.tasks.CancellationTokenSource
import kotlinx.coroutines.tasks.await

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
            val cts = CancellationTokenSource()
            val location =
                client
                    .getCurrentLocation(Priority.PRIORITY_BALANCED_POWER_ACCURACY, cts.token)
                    .await()
            location?.let { LocationResult(it.latitude, it.longitude) }
        } catch (
            @Suppress("TooGenericExceptionCaught") _: Exception,
        ) {
            null
        }
    }
}
