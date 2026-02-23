package com.poziomki.app.location

data class LocationResult(
    val latitude: Double,
    val longitude: Double,
)

expect class LocationProvider {
    suspend fun getCurrentLocation(): LocationResult?

    fun isPermissionGranted(): Boolean
}
