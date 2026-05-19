package com.poziomki.app.observability

import com.google.firebase.crashlytics.FirebaseCrashlytics

class FirebaseCrashReporter : CrashReporter {
    private val crashlytics by lazy { FirebaseCrashlytics.getInstance() }

    override fun recordNonFatal(
        throwable: Throwable,
        tags: Map<String, String>,
    ) {
        tags.forEach { (k, v) -> crashlytics.setCustomKey(k, v) }
        crashlytics.recordException(throwable)
    }

    override fun setUserId(id: String?) {
        crashlytics.setUserId(id.orEmpty())
    }

    override fun log(message: String) {
        crashlytics.log(message)
    }
}
