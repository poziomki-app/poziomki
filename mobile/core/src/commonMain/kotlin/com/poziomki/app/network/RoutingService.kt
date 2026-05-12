package com.poziomki.app.network

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.defaultRequest
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.http.HttpHeaders
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement

@Serializable
private data class OsrmResponse(
    val code: String,
    val routes: List<OsrmRoute> = emptyList(),
)

@Serializable
private data class OsrmRoute(
    val geometry: JsonElement,
    val distance: Double,
    val duration: Double,
)

data class WalkingRoute(
    val geometryJson: String,
    val distanceMeters: Double,
    val durationSeconds: Double,
)

class RoutingService(
    engine: HttpClientEngine,
) {
    private val json =
        Json {
            ignoreUnknownKeys = true
            isLenient = true
        }
    private val client =
        HttpClient(engine) {
            install(ContentNegotiation) { json(json) }
            defaultRequest {
                url("https://router.project-osrm.org/")
                header(HttpHeaders.UserAgent, "Poziomki/1.0")
            }
        }

    private val cache = mutableMapOf<String, WalkingRoute>()

    suspend fun walkingRoute(
        fromLat: Double,
        fromLng: Double,
        toLat: Double,
        toLng: Double,
    ): WalkingRoute? {
        val key = "$fromLat,$fromLng->$toLat,$toLng"
        cache[key]?.let { return it }
        return try {
            val path = "route/v1/foot/$fromLng,$fromLat;$toLng,$toLat"
            val resp: OsrmResponse =
                client
                    .get(path) {
                        url { parameters.append("overview", "full") }
                        url { parameters.append("geometries", "geojson") }
                    }.body()
            if (resp.code != "Ok") return null
            val route = resp.routes.firstOrNull() ?: return null
            val result =
                WalkingRoute(
                    geometryJson = json.encodeToString(JsonElement.serializer(), route.geometry),
                    distanceMeters = route.distance,
                    durationSeconds = route.duration,
                )
            cache[key] = result
            result
        } catch (_: Exception) {
            null
        }
    }
}
