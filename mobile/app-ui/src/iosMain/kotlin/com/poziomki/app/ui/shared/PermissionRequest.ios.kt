package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import kotlinx.cinterop.ExperimentalForeignApi
import platform.CoreLocation.CLAuthorizationStatus
import platform.CoreLocation.CLLocationManager
import platform.CoreLocation.CLLocationManagerDelegateProtocol
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedAlways
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedWhenInUse
import platform.CoreLocation.kCLAuthorizationStatusNotDetermined
import platform.darwin.NSObject

@OptIn(ExperimentalForeignApi::class)
@Composable
actual fun rememberLocationPermissionLauncher(onResult: (Boolean) -> Unit): () -> Unit {
    val state =
        remember {
            object {
                val manager = CLLocationManager()
                var delegate: CLLocationManagerDelegateProtocol? = null
            }
        }

    DisposableEffect(Unit) {
        onDispose {
            state.manager.delegate = null
            state.delegate = null
        }
    }

    return {
        val status = CLLocationManager.authorizationStatus()
        if (status == kCLAuthorizationStatusAuthorizedWhenInUse ||
            status == kCLAuthorizationStatusAuthorizedAlways
        ) {
            onResult(true)
        } else if (status != kCLAuthorizationStatusNotDetermined) {
            onResult(false)
        } else {
            val delegate =
                object : NSObject(), CLLocationManagerDelegateProtocol {
                    override fun locationManager(
                        manager: CLLocationManager,
                        didChangeAuthorizationStatus: CLAuthorizationStatus,
                    ) {
                        if (didChangeAuthorizationStatus == kCLAuthorizationStatusNotDetermined) return
                        val granted =
                            didChangeAuthorizationStatus == kCLAuthorizationStatusAuthorizedWhenInUse ||
                                didChangeAuthorizationStatus == kCLAuthorizationStatusAuthorizedAlways
                        manager.delegate = null
                        state.delegate = null
                        onResult(granted)
                    }
                }
            state.delegate = delegate
            state.manager.delegate = delegate
            state.manager.requestWhenInUseAuthorization()
        }
    }
}
