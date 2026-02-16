package com.poziomki.app.api

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.defaultRequest
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import io.ktor.http.HttpHeaders
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
internal data class PhotonResponse(
    val features: List<PhotonFeature> = emptyList(),
)

@Serializable
internal data class PhotonFeature(
    val geometry: PhotonGeometry,
    val properties: PhotonProperties,
)

@Serializable
internal data class PhotonGeometry(
    val coordinates: List<Double>,
)

@Serializable
internal data class PhotonProperties(
    val name: String? = null,
    val street: String? = null,
    val housenumber: String? = null,
    val city: String? = null,
    val state: String? = null,
    val country: String? = null,
)

class GeocodingService(
    engine: HttpClientEngine,
) {
    private val client =
        HttpClient(engine) {
            install(ContentNegotiation) {
                json(
                    Json {
                        ignoreUnknownKeys = true
                        isLenient = true
                    },
                )
            }
            defaultRequest {
                url("https://photon.komoot.io/")
                header(HttpHeaders.UserAgent, "Poziomki/1.0")
            }
        }

    suspend fun search(query: String): List<GeocodingResult> =
        try {
            val response: PhotonResponse =
                client
                    .get("api") {
                        parameter("q", query)
                        parameter("limit", "5")
                        parameter("lang", "pl")
                        parameter("lat", "52.2297")
                        parameter("lon", "21.0122")
                        parameter("location_bias_scale", "0.5")
                        parameter("bbox", "20.75,52.05,21.35,52.45")
                    }.body()
            response.features.mapNotNull { it.toResult() }
        } catch (_: Exception) {
            emptyList()
        }

    suspend fun reverse(
        lat: Double,
        lng: Double,
    ): String? =
        try {
            val response: PhotonResponse =
                client
                    .get("reverse") {
                        parameter("lat", lat)
                        parameter("lon", lng)
                        parameter("lang", "pl")
                    }.body()
            response.features
                .firstOrNull()
                ?.toResult()
                ?.name
        } catch (_: Exception) {
            null
        }
}

private fun PhotonFeature.toResult(): GeocodingResult? {
    if (geometry.coordinates.size < 2) return null
    val lng = geometry.coordinates[0]
    val lat = geometry.coordinates[1]
    val p = properties
    val name =
        buildList {
            p.name?.let { add(it) }
            if (p.street != null) {
                val street = if (p.housenumber != null) "${p.street} ${p.housenumber}" else p.street
                if (street != p.name) add(street)
            }
            p.city?.let { if (it != p.name) add(it) }
        }.joinToString(", ").ifBlank { p.country ?: return null }
    return GeocodingResult(name = name, lat = lat, lng = lng)
}
