package com.poziomki.app.chat

import kotlin.concurrent.Volatile

object ActiveChat {
    @Volatile
    var roomId: String? = null
}
