package com.poziomki.app.ui.shared

import coil3.intercept.Interceptor
import coil3.request.ImageResult

/**
 * Coil interceptor that stabilizes cache keys for imgproxy signed URLs.
 *
 * Imgproxy URLs look like: `{base}/img/{sig}/{expiry}/{variant}.{fmt}/{filename}`
 * The signature and expiry change on every API response, which makes Coil treat
 * each response as a cache miss even though the underlying image is the same.
 *
 * This interceptor extracts the stable tail (`{variant}.{fmt}/{filename}`) and
 * sets it as both the memory and disk cache key so caches survive across
 * refreshes and cold starts.
 */
class ImgproxyCacheInterceptor : Interceptor {
    override suspend fun intercept(chain: Interceptor.Chain): ImageResult {
        val data = chain.request.data
        val stableKey = if (data is String) extractStableKey(data) else null
        if (stableKey != null) {
            val newRequest =
                chain.request
                    .newBuilder()
                    .memoryCacheKey(stableKey)
                    .diskCacheKey(stableKey)
                    .build()
            return chain.withRequest(newRequest).proceed()
        }
        return chain.proceed()
    }

    private fun extractStableKey(url: String): String? {
        val idx = url.indexOf("/img/")
        if (idx < 0) return null
        // path after /img/ = {sig}/{expiry}/{variant}.{fmt}/{filename}
        val segments = url.substring(idx + 5).split('/')
        if (segments.size < 3) return null
        // last two segments are stable: variant.fmt / filename
        return segments[segments.size - 2] + "/" + segments[segments.size - 1]
    }
}
