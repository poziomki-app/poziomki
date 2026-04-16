package com.poziomki.app.network

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.defaultRequest
import io.ktor.client.request.get
import io.ktor.client.request.parameter
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
internal data class OpenMeteoResponse(
    val current: OpenMeteoCurrent,
)

@Serializable
internal data class OpenMeteoCurrent(
    @SerialName("temperature_2m") val temperatureC: Double,
    @SerialName("weather_code") val weatherCode: Int,
    @SerialName("wind_speed_10m") val windSpeedKmh: Double,
)

data class WeatherInfo(
    val temperatureC: Int,
    val windSpeedKmh: Int,
    val emoji: String,
    val description: String,
)

class WeatherService(
    engine: HttpClientEngine,
) {
    private val client =
        HttpClient(engine) {
            install(ContentNegotiation) {
                json(Json { ignoreUnknownKeys = true })
            }
            defaultRequest {
                url("https://api.open-meteo.com/")
            }
        }

    suspend fun getWarsawWeather(): WeatherInfo? =
        try {
            val response: OpenMeteoResponse =
                client
                    .get("v1/forecast") {
                        parameter("latitude", "52.23")
                        parameter("longitude", "21.01")
                        parameter("current", "temperature_2m,weather_code,wind_speed_10m")
                        parameter("wind_speed_unit", "kmh")
                    }.body()
            val c = response.current
            val (emoji, desc) = weatherCodeToLabel(c.weatherCode)
            WeatherInfo(
                temperatureC = c.temperatureC.toInt(),
                windSpeedKmh = c.windSpeedKmh.toInt(),
                emoji = emoji,
                description = desc,
            )
        } catch (_: Exception) {
            null
        }

    private fun weatherCodeToLabel(code: Int): Pair<String, String> =
        when (code) {
            0 -> "☀️" to "bezchmurnie"
            1 -> "🌤️" to "przeważnie jasne"
            2 -> "⛅" to "częściowe zachmurzenie"
            3 -> "☁️" to "zachmurzenie"
            in 45..48 -> "🌫️" to "mgła"
            in 51..55 -> "🌦️" to "mżawka"
            in 61..65 -> "🌧️" to "deszcz"
            in 71..77 -> "❄️" to "śnieg"
            in 80..82 -> "🌦️" to "przelotny deszcz"
            in 85..86 -> "🌨️" to "śnieżyca"
            in 95..99 -> "⛈️" to "burza"
            else -> "🌡️" to "nieznane"
        }
}
