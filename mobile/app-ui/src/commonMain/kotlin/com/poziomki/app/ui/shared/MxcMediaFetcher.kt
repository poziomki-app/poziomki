package com.poziomki.app.ui.shared

import coil3.ImageLoader
import coil3.Uri
import coil3.fetch.Fetcher
import coil3.request.Options

class MxcMediaFetcher {
    class Factory : Fetcher.Factory<Uri> {
        override fun create(
            data: Uri,
            options: Options,
            imageLoader: ImageLoader,
        ): Fetcher? {
            // No longer needed — chat images are regular HTTP URLs via imgproxy.
            // Return null so Coil handles them with the default network fetcher.
            return null
        }
    }
}
