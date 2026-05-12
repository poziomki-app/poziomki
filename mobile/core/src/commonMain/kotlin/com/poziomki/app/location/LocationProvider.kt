package com.poziomki.app.location

import kotlinx.coroutines.flow.Flow

data class LocationResult(
    val latitude: Double,
    val longitude: Double,
)

expect class LocationProvider {
    suspend fun getCurrentLocation(): LocationResult?

    fun locationUpdates(): Flow<LocationResult>

    fun isPermissionGranted(): Boolean
}
