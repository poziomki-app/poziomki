package com.poziomki.app.util

private const val API_BASE_URL = "http://localhost:3000"

fun resolveImageUrl(url: String): String = if (url.startsWith("/")) "$API_BASE_URL$url" else url

fun isImageUrl(value: String): Boolean = value.startsWith("/") || value.startsWith("http://") || value.startsWith("https://")
