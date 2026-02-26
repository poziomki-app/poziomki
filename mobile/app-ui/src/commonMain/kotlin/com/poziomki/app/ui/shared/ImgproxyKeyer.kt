package com.poziomki.app.ui.shared

import coil3.Uri
import coil3.key.Keyer
import coil3.request.Options

/**
 * Coil cache-key override for imgproxy signed URLs.
 *
 * Imgproxy URLs look like: `{base}/img/{sig}/{expiry}/{variant}.{fmt}/{filename}`
 * The signature and expiry change on every API response, which makes Coil treat
 * each response as a cache miss even though the underlying image is the same.
 *
 * This keyer extracts the stable tail (`{variant}.{fmt}/{filename}`) and uses it
 * as the cache key so that memory and disk caches survive across refreshes.
 */
class ImgproxyKeyer : Keyer<Uri> {
    override fun key(data: Uri, options: Options): String? {
        val path = data.path ?: return null
        val idx = path.indexOf("/img/")
        if (idx < 0) return null
        // path after /img/ = {sig}/{expiry}/{variant}.{fmt}/{filename}
        val segments = path.substring(idx + 5).split('/')
        if (segments.size < 3) return null
        // last two segments are stable: variant.fmt / filename
        return segments[segments.size - 2] + "/" + segments[segments.size - 1]
    }
}
