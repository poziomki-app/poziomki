package com.poziomki.app.location

import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.useContents
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import platform.CoreLocation.CLAuthorizationStatus
import platform.CoreLocation.CLLocation
import platform.CoreLocation.CLLocationManager
import platform.CoreLocation.CLLocationManagerDelegateProtocol
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedAlways
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedWhenInUse
import platform.CoreLocation.kCLLocationAccuracyHundredMeters
import platform.Foundation.NSError
import platform.darwin.NSObject
import kotlin.coroutines.resume

private const val LOCATION_TIMEOUT_MS = 5_000L

@OptIn(ExperimentalForeignApi::class)
actual class LocationProvider {
    private val manager =
        CLLocationManager().apply {
            desiredAccuracy = kCLLocationAccuracyHundredMeters
        }

    actual fun isPermissionGranted(): Boolean {
        val status: CLAuthorizationStatus = CLLocationManager.authorizationStatus()
        return status == kCLAuthorizationStatusAuthorizedWhenInUse ||
            status == kCLAuthorizationStatusAuthorizedAlways
    }

    actual suspend fun getCurrentLocation(): LocationResult? {
        if (!isPermissionGranted()) return null
        return withTimeoutOrNull(LOCATION_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                val delegate =
                    object : NSObject(), CLLocationManagerDelegateProtocol {
                        override fun locationManager(
                            manager: CLLocationManager,
                            didUpdateLocations: List<*>,
                        ) {
                            val location = didUpdateLocations.lastOrNull() as? CLLocation ?: return
                            manager.stopUpdatingLocation()
                            manager.delegate = null
                            if (continuation.isActive) {
                                location.coordinate.useContents {
                                    continuation.resume(LocationResult(latitude, longitude))
                                }
                            }
                        }

                        override fun locationManager(
                            manager: CLLocationManager,
                            didFailWithError: NSError,
                        ) {
                            manager.stopUpdatingLocation()
                            manager.delegate = null
                            if (continuation.isActive) continuation.resume(null)
                        }
                    }
                manager.delegate = delegate
                continuation.invokeOnCancellation {
                    manager.stopUpdatingLocation()
                    manager.delegate = null
                }
                manager.startUpdatingLocation()
            }
        }
    }

    actual fun locationUpdates(): Flow<LocationResult> =
        callbackFlow {
            if (!isPermissionGranted()) {
                close()
                return@callbackFlow
            }
            val delegate =
                object : NSObject(), CLLocationManagerDelegateProtocol {
                    override fun locationManager(
                        manager: CLLocationManager,
                        didUpdateLocations: List<*>,
                    ) {
                        val location = didUpdateLocations.lastOrNull() as? CLLocation ?: return
                        location.coordinate.useContents {
                            trySend(LocationResult(latitude, longitude))
                        }
                    }

                    override fun locationManager(
                        manager: CLLocationManager,
                        didFailWithError: NSError,
                    ) {
                        // Keep flow open; transient errors shouldn't kill subscription.
                    }
                }
            manager.delegate = delegate
            manager.startUpdatingLocation()
            awaitClose {
                manager.stopUpdatingLocation()
                manager.delegate = null
            }
        }
}
