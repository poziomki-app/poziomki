package com.poziomki.app.network

fun MatrixRoomResolveData.resolveRoomId(): String? =
    roomId
        ?.takeIf { it.startsWith("!") }
        ?: roomIdSnakeCase?.takeIf { it.startsWith("!") }
