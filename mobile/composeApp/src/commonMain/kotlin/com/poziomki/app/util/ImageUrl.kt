package com.poziomki.app.util

import org.koin.mp.KoinPlatform

private val apiBaseUrl: String by lazy {
    KoinPlatform.getKoin().getProperty("API_BASE_URL", "http://localhost:5150")
}

fun resolveImageUrl(url: String): String = if (url.startsWith("/")) "$apiBaseUrl$url" else url

fun isImageUrl(value: String): Boolean = value.startsWith("/") || value.startsWith("https://")
