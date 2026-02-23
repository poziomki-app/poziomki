package com.poziomki.app.location

actual class LocationProvider {
    actual fun isPermissionGranted(): Boolean = false

    actual suspend fun getCurrentLocation(): LocationResult? = null
}
