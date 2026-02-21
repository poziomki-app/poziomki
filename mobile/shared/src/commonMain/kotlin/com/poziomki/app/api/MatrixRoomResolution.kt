package com.poziomki.app.api

fun MatrixRoomResolveData.resolveRoomId(): String? =
    roomId
        ?.takeIf { it.startsWith("!") }
        ?: roomIdSnakeCase?.takeIf { it.startsWith("!") }
