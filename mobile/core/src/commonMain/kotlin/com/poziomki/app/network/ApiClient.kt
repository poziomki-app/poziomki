package com.poziomki.app.network

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.cookies.HttpCookies
import io.ktor.client.plugins.defaultRequest
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.delete
import io.ktor.client.request.forms.MultiPartFormDataContent
import io.ktor.client.request.forms.formData
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.patch
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.client.statement.HttpResponse
import io.ktor.client.statement.bodyAsChannel
import io.ktor.http.ContentType
import io.ktor.http.Headers
import io.ktor.http.HttpHeaders
import io.ktor.http.contentType
import io.ktor.http.isSuccess
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.core.readBytes
import io.ktor.utils.io.readRemaining
import kotlinx.serialization.json.Json

class ApiClient(
    baseUrl: String,
    engine: io.ktor.client.engine.HttpClientEngine,
    enableHttpLogging: Boolean = false,
    @PublishedApi internal val tokenProvider: suspend () -> String?,
    @PublishedApi internal val onUnauthorized: (suspend () -> Unit)? = null,
) {
    private val json =
        Json {
            ignoreUnknownKeys = true
            isLenient = true
            encodeDefaults = true
            explicitNulls = false
        }

    @PublishedApi
    internal val httpClient =
        HttpClient(engine) {
            install(ContentNegotiation) {
                json(json)
            }
            install(HttpCookies)
            if (enableHttpLogging) {
                install(Logging) {
                    level = LogLevel.INFO
                    sanitizeHeader { header -> header == HttpHeaders.Authorization }
                }
            }
            defaultRequest {
                url(baseUrl)
                header("X-Image-Format", preferredImageFormat())
            }
        }

    suspend inline fun <reified T> get(path: String): ApiResult<T> =
        request {
            httpClient.get(path) {
                contentType(ContentType.Application.Json)
                tokenProvider()?.let { bearerAuth(it) }
            }
        }

    suspend inline fun <reified T> post(
        path: String,
        body: Any? = null,
    ): ApiResult<T> =
        request {
            httpClient.post(path) {
                contentType(ContentType.Application.Json)
                tokenProvider()?.let { bearerAuth(it) }
                body?.let { setBody(it) }
            }
        }

    suspend inline fun <reified T> patch(
        path: String,
        body: Any? = null,
    ): ApiResult<T> =
        request {
            httpClient.patch(path) {
                contentType(ContentType.Application.Json)
                tokenProvider()?.let { bearerAuth(it) }
                body?.let { setBody(it) }
            }
        }

    suspend inline fun <reified T> delete(
        path: String,
        body: Any? = null,
    ): ApiResult<T> =
        request {
            httpClient.delete(path) {
                contentType(ContentType.Application.Json)
                tokenProvider()?.let { bearerAuth(it) }
                body?.let { setBody(it) }
            }
        }

    suspend fun uploadFile(
        bytes: ByteArray,
        fileName: String,
        context: String = "profile_gallery",
    ): ApiResult<UploadResponse> =
        try {
            val mimeType = detectMimeType(bytes)
            val response =
                httpClient.post("/api/v1/uploads") {
                    tokenProvider()?.let { bearerAuth(it) }
                    setBody(
                        MultiPartFormDataContent(
                            formData {
                                append(
                                    "file",
                                    bytes,
                                    Headers.build {
                                        append(HttpHeaders.ContentType, mimeType)
                                        append(HttpHeaders.ContentDisposition, "filename=\"$fileName\"")
                                    },
                                )
                                append("context", context)
                            },
                        ),
                    )
                }
            if (response.status.isSuccess()) {
                val wrapper = response.body<ApiResponse<UploadResponse>>()
                if (wrapper.data != null) {
                    ApiResult.Success(wrapper.data)
                } else {
                    ApiResult.Error("No data", "NOT_FOUND", 404)
                }
            } else {
                if (response.status.value == 401) {
                    onUnauthorized?.invoke()
                }
                val error =
                    try {
                        response.body<ApiErrorResponse>()
                    } catch (_: Exception) {
                        ApiErrorResponse(error = "Upload failed", code = response.status.value.toString())
                    }
                ApiResult.Error(error.error, error.code, response.status.value)
            }
        } catch (e: Exception) {
            ApiResult.Error(e.message ?: "Upload error", "NETWORK_ERROR", 0)
        }

    private fun detectMimeType(bytes: ByteArray): String {
        if (bytes.size < 8) return "image/jpeg"
        return when {
            bytes[0] == 0xFF.toByte() && bytes[1] == 0xD8.toByte() -> "image/jpeg"

            bytes[0] == 0x89.toByte() && bytes[1] == 0x50.toByte() -> "image/png"

            bytes.size >= 12 &&
                bytes[0] == 0x52.toByte() && bytes[8] == 0x57.toByte() -> "image/webp"

            else -> "image/jpeg"
        }
    }

    suspend fun downloadBytes(path: String): ApiResult<ByteArray> =
        try {
            val response =
                httpClient.get(path) {
                    tokenProvider()?.let { bearerAuth(it) }
                }
            if (response.status.isSuccess()) {
                val channel = response.bodyAsChannel()

                @Suppress("DEPRECATION")
                val bytes = channel.readRemaining().readBytes()
                ApiResult.Success(bytes)
            } else {
                if (response.status.value == 401) {
                    onUnauthorized?.invoke()
                }
                ApiResult.Error("Export failed", response.status.value.toString(), response.status.value)
            }
        } catch (e: Exception) {
            ApiResult.Error("Brak po\u0142\u0105czenia z internetem", "NETWORK_ERROR", 0)
        }

    @PublishedApi
    internal suspend inline fun <reified T> request(block: () -> HttpResponse): ApiResult<T> =
        try {
            val response = block()
            if (response.status.isSuccess()) {
                val wrapper = response.body<ApiResponse<T>>()
                if (wrapper.data != null) {
                    ApiResult.Success(wrapper.data)
                } else {
                    ApiResult.Error("No data", "NOT_FOUND", 404)
                }
            } else {
                if (response.status.value == 401) {
                    onUnauthorized?.invoke()
                }
                val error =
                    try {
                        response.body<ApiErrorResponse>()
                    } catch (_: Exception) {
                        ApiErrorResponse(
                            error = "Co\u015b posz\u0142o nie tak. Spr\u00f3buj ponownie.",
                            code = response.status.value.toString(),
                            requestId = null,
                        )
                    }
                ApiResult.Error(error.error, error.code, response.status.value)
            }
        } catch (e: Exception) {
            ApiResult.Error("Brak po\u0142\u0105czenia z internetem", "NETWORK_ERROR", 0)
        }
}
