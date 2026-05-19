package com.poziomki.app.observability

interface CrashReporter {
    fun recordNonFatal(
        throwable: Throwable,
        tags: Map<String, String> = emptyMap(),
    )

    fun setUserId(id: String?)

    fun log(message: String)
}

object NoopCrashReporter : CrashReporter {
    override fun recordNonFatal(
        throwable: Throwable,
        tags: Map<String, String>,
    ) = Unit

    override fun setUserId(id: String?) = Unit

    override fun log(message: String) = Unit
}
