package com.poziomki.app.api

private val LEGACY_MATRIX_FALLBACK_STATUSES = setOf(404, 405, 501)

fun MatrixRoomResolveData.resolveRoomId(): String? =
    roomId
        ?.takeIf { it.startsWith("!") }
        ?: roomIdSnakeCase?.takeIf { it.startsWith("!") }

fun ApiResult.Error.supportsLegacyMatrixFallback(): Boolean = status in LEGACY_MATRIX_FALLBACK_STATUSES
