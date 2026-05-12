package com.poziomki.app.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import androidx.core.content.ContextCompat
import androidx.core.location.LocationListenerCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.location.LocationRequestCompat
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

actual class LocationProvider(
    private val context: Context,
) {
    actual fun isPermissionGranted(): Boolean =
        hasPermission(Manifest.permission.ACCESS_COARSE_LOCATION) ||
            hasPermission(Manifest.permission.ACCESS_FINE_LOCATION)

    private fun hasPermission(perm: String): Boolean =
        ContextCompat.checkSelfPermission(
            context,
            perm,
        ) == PackageManager.PERMISSION_GRANTED

    actual suspend fun getCurrentLocation(): LocationResult? {
        if (!isPermissionGranted()) {
            Log.w(TAG, "permission not granted")
            return null
        }
        Log.i(TAG, "getCurrentLocation start (fine=${hasPermission(Manifest.permission.ACCESS_FINE_LOCATION)})")

        // 1. Fused last-known — instant on GMS/microG devices.
        if (isFusedLikelyAvailable()) {
            fusedLastLocation()?.let {
                Log.i(TAG, "fused last fix=$it")
                return it
            }
        }
        // 2. Platform last-known — works on degoogled phones if GPS/NETWORK
        //    has any cached fix. This is the "Organic Maps shows dot instantly" path.
        platformLastKnownSync(context.getSystemService(LocationManager::class.java))?.let {
            Log.i(TAG, "platform last fix=$it")
            return it
        }
        // 3. Fused current — requires GMS/microG, but worth it when it works.
        if (isFusedLikelyAvailable()) {
            fusedCurrentLocation()?.let {
                Log.i(TAG, "fused current fix=$it")
                return it
            }
        }
        // 4. Platform one-shot — listen on GPS_PROVIDER (if fine) and
        //    NETWORK_PROVIDER in parallel; first fix wins.
        platformOneShot()?.let {
            Log.i(TAG, "platform one-shot fix=$it")
            return it
        }
        Log.w(TAG, "no fix obtained")
        return null
    }

    actual fun locationUpdates(): Flow<LocationResult> =
        callbackFlow {
            if (!isPermissionGranted()) {
                close()
                return@callbackFlow
            }
            val manager = context.getSystemService(LocationManager::class.java)
            val listeners = mutableListOf<LocationListenerCompat>()

            fun emit(location: Location) {
                trySend(location.toLocationResult())
            }

            // Seed with last-known so UI gets a dot immediately, even before first fresh fix.
            platformLastKnownSync(manager)?.let { trySend(it) }

            if (manager != null) {
                val request =
                    LocationRequestCompat
                        .Builder(UPDATE_INTERVAL_MS)
                        .setMinUpdateIntervalMillis(UPDATE_INTERVAL_MS)
                        .setMinUpdateDistanceMeters(UPDATE_MIN_DISTANCE_M)
                        .setQuality(LocationRequestCompat.QUALITY_HIGH_ACCURACY)
                        .build()
                // Prefer GPS strictly when fine permission is granted — like
                // Organic Maps. NETWORK_PROVIDER returns cell-tower triangulation
                // (center of Warsaw / Marszałkowska on Polish networks), which is
                // wrong if we have GPS. Only fall back to NETWORK if GPS is off.
                val useGps =
                    hasPermission(Manifest.permission.ACCESS_FINE_LOCATION) &&
                        manager.isProviderEnabledSafe(LocationManager.GPS_PROVIDER)
                val providers = mutableListOf<String>()
                if (useGps) {
                    providers += LocationManager.GPS_PROVIDER
                } else if (manager.isProviderEnabledSafe(LocationManager.NETWORK_PROVIDER)) {
                    providers += LocationManager.NETWORK_PROVIDER
                }
                for (provider in providers) {
                    val listener = LocationListenerCompat { emit(it) }
                    listeners += listener
                    runCatching {
                        @Suppress("MissingPermission")
                        LocationManagerCompat.requestLocationUpdates(
                            manager,
                            provider,
                            request,
                            listener,
                            Looper.getMainLooper(),
                        )
                    }.onFailure { Log.w(TAG, "updates failed for $provider", it) }
                }
            }

            awaitClose {
                if (manager != null) {
                    for (l in listeners) LocationManagerCompat.removeUpdates(manager, l)
                }
            }
        }

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
    private suspend fun fusedLastLocation(): LocationResult? =
        try {
            val client = LocationServices.getFusedLocationProviderClient(context)
            withTimeoutOrNull(FUSED_LAST_TIMEOUT_MS) {
                suspendCancellableCoroutine { continuation ->
                    client.lastLocation
                        .addOnSuccessListener { loc ->
                            if (continuation.isActive) continuation.resume(loc?.toLocationResult())
                        }.addOnFailureListener {
                            if (continuation.isActive) continuation.resume(null)
                        }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "fused last threw", e)
            null
        }

    @Suppress("MissingPermission", "TooGenericExceptionCaught")
    private suspend fun fusedCurrentLocation(): LocationResult? =
        try {
            val client = LocationServices.getFusedLocationProviderClient(context)
            withTimeoutOrNull(FUSED_CURRENT_TIMEOUT_MS) {
                suspendCancellableCoroutine { continuation ->
                    client
                        .getCurrentLocation(Priority.PRIORITY_BALANCED_POWER_ACCURACY, null)
                        .addOnSuccessListener { loc ->
                            if (continuation.isActive) continuation.resume(loc?.toLocationResult())
                        }.addOnFailureListener { e ->
                            Log.w(TAG, "fused current failed", e)
                            if (continuation.isActive) continuation.resume(null)
                        }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "fused current threw", e)
            null
        }

    @Suppress("MissingPermission")
    private fun platformLastKnownSync(manager: LocationManager?): LocationResult? {
        if (manager == null) return null
        // Build provider list in trust order: GPS first, NETWORK only as last
        // resort (it returns cell-tower triangulation, often wildly off).
        // PASSIVE only as a tiebreaker if everything else is unavailable.
        val providers = mutableListOf<String>()
        if (hasPermission(Manifest.permission.ACCESS_FINE_LOCATION)) providers += LocationManager.GPS_PROVIDER
        val now = SystemClock.elapsedRealtimeNanos()

        // Try preferred providers first — return the first fresh-enough fix.
        for (p in providers) {
            val loc =
                runCatching {
                    if (manager.isProviderEnabledSafe(p)) manager.getLastKnownLocation(p) else null
                }.getOrNull() ?: continue
            val ageNs = now - loc.elapsedRealtimeNanos
            if (ageNs <= LAST_KNOWN_MAX_AGE_NS) return loc.toLocationResult()
        }
        // No GPS last-known — try NETWORK only if it's *very* fresh AND
        // reasonably accurate (< 500 m), otherwise we'd pin the user to
        // a cell-tower center.
        runCatching {
            if (manager.isProviderEnabledSafe(LocationManager.NETWORK_PROVIDER)) {
                manager.getLastKnownLocation(LocationManager.NETWORK_PROVIDER)
            } else {
                null
            }
        }.getOrNull()?.let { loc ->
            val ageNs = now - loc.elapsedRealtimeNanos
            val accuracyOk = !loc.hasAccuracy() || loc.accuracy <= NETWORK_ACCURACY_MAX_M
            if (ageNs <= NETWORK_LAST_KNOWN_MAX_AGE_NS && accuracyOk) {
                return loc.toLocationResult()
            }
        }
        return null
    }

    private suspend fun platformOneShot(): LocationResult? {
        val manager = context.getSystemService(LocationManager::class.java) ?: return null
        // Same trust order: GPS only when we have fine permission; NETWORK
        // strictly as fallback when GPS is unavailable.
        val useGps =
            hasPermission(Manifest.permission.ACCESS_FINE_LOCATION) &&
                manager.isProviderEnabledSafe(LocationManager.GPS_PROVIDER)
        val providers = mutableListOf<String>()
        if (useGps) {
            providers += LocationManager.GPS_PROVIDER
        } else if (manager.isProviderEnabledSafe(LocationManager.NETWORK_PROVIDER)) {
            providers += LocationManager.NETWORK_PROVIDER
        }
        if (providers.isEmpty()) return null

        return withTimeoutOrNull(PLATFORM_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                val listeners = mutableListOf<LocationListenerCompat>()

                fun removeAll() {
                    for (l in listeners) LocationManagerCompat.removeUpdates(manager, l)
                }
                val request =
                    LocationRequestCompat
                        .Builder(PLATFORM_TIMEOUT_MS)
                        .setQuality(LocationRequestCompat.QUALITY_BALANCED_POWER_ACCURACY)
                        .setMaxUpdates(1)
                        .setDurationMillis(PLATFORM_TIMEOUT_MS)
                        .build()
                for (p in providers) {
                    val listener =
                        object : LocationListenerCompat {
                            override fun onLocationChanged(location: Location) {
                                removeAll()
                                if (continuation.isActive) {
                                    continuation.resume(location.toLocationResult())
                                }
                            }
                        }
                    listeners += listener
                    runCatching {
                        @Suppress("MissingPermission")
                        LocationManagerCompat.requestLocationUpdates(
                            manager,
                            p,
                            request,
                            listener,
                            Looper.getMainLooper(),
                        )
                    }.onFailure { Log.w(TAG, "one-shot requestUpdates failed for $p", it) }
                }
                continuation.invokeOnCancellation { removeAll() }
            }
        }
    }

    private fun LocationManager.isProviderEnabledSafe(provider: String): Boolean =
        runCatching { isProviderEnabled(provider) }.getOrDefault(false)

    private companion object {
        const val TAG = "PoziomkiLocation"
        const val GMS_PACKAGE = "com.google.android.gms"
        const val FUSED_LAST_TIMEOUT_MS = 1_500L
        const val FUSED_CURRENT_TIMEOUT_MS = 8_000L
        const val PLATFORM_TIMEOUT_MS = 15_000L
        const val UPDATE_INTERVAL_MS = 5_000L
        const val UPDATE_MIN_DISTANCE_M = 5f
        const val LAST_KNOWN_MAX_AGE_NS = 5L * 60 * 1_000_000_000 // 5 minutes (GPS)
        const val NETWORK_LAST_KNOWN_MAX_AGE_NS = 60L * 1_000_000_000 // 1 minute
        const val NETWORK_ACCURACY_MAX_M = 500f
    }
}

private fun Location.toLocationResult(): LocationResult = LocationResult(latitude, longitude)
