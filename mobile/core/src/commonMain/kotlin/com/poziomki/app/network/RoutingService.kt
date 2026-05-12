package com.poziomki.app.network

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

data class WalkingRoute(
    val geometryJson: String,
    val distanceMeters: Double,
    val durationSeconds: Double,
)

/**
 * Resolves walking routes by calling our own backend, which proxies to a
 * self-hosted OSRM instance. Coordinates never leave Poziomki infra.
 */
class RoutingService(
    private val apiService: ApiService,
) {
    private val cacheMutex = Mutex()

    // mutableMapOf preserves insertion order on KMP; we evict the oldest
    // entry once we exceed CACHE_CAPACITY. Good enough for a UI cache.
    private val cache = mutableMapOf<String, WalkingRoute>()

    suspend fun walkingRoute(
        fromLat: Double,
        fromLng: Double,
        toLat: Double,
        toLng: Double,
    ): WalkingRoute? {
        val key = "$fromLat,$fromLng->$toLat,$toLng"
        cacheMutex.withLock { cache[key] }?.let { return it }
        val result =
            when (val r = apiService.walkingRoute(fromLat, fromLng, toLat, toLng)) {
                is ApiResult.Success -> {
                    WalkingRoute(
                        geometryJson = r.data.geometryJson,
                        distanceMeters = r.data.distanceMeters,
                        durationSeconds = r.data.durationSeconds,
                    )
                }

                is ApiResult.Error -> {
                    return null
                }
            }
        cacheMutex.withLock {
            cache[key] = result
            while (cache.size > CACHE_CAPACITY) {
                val eldest = cache.keys.iterator().next()
                cache.remove(eldest)
            }
        }
        return result
    }

    private companion object {
        const val CACHE_CAPACITY = 64
    }
}
