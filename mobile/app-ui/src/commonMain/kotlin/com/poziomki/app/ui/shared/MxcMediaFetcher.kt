package com.poziomki.app.ui.shared

import coil3.ImageLoader
import coil3.Uri
import coil3.decode.DataSource
import coil3.decode.ImageSource
import coil3.fetch.FetchResult
import coil3.fetch.Fetcher
import coil3.fetch.SourceFetchResult
import coil3.request.Options
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull
import okio.Buffer
import okio.FileSystem
import okio.IOException

private const val THUMBNAIL_SIZE = 256L
private const val CLIENT_READY_TIMEOUT_MS = 15_000L
private const val IN_MEMORY_MXC_CACHE_MAX_ENTRIES = 256

class MxcMediaFetcher(
    private val matrixClient: MatrixClient,
    private val mxcUrl: String,
) : Fetcher {
    override suspend fun fetch(): FetchResult {
        MxcMediaCache.get(mxcUrl)?.let { cached ->
            val cachedBuffer = Buffer().apply { write(cached) }
            return SourceFetchResult(
                source = ImageSource(source = cachedBuffer, fileSystem = FileSystem.SYSTEM),
                mimeType = null,
                dataSource = DataSource.NETWORK,
            )
        }

        withTimeoutOrNull(CLIENT_READY_TIMEOUT_MS) {
            matrixClient.state.first { it is MatrixClientState.Ready }
        } ?: throw IOException("Matrix client not ready for media fetch")

        val bytes =
            matrixClient.getMediaThumbnail(mxcUrl, THUMBNAIL_SIZE, THUMBNAIL_SIZE)
                ?: matrixClient.getMediaContent(mxcUrl)
                ?: throw IOException("Matrix media not available for $mxcUrl")
        if (bytes.isEmpty()) throw IOException("Matrix media empty for $mxcUrl")
        MxcMediaCache.put(mxcUrl, bytes)
        val buffer = Buffer().apply { write(bytes) }
        return SourceFetchResult(
            source = ImageSource(source = buffer, fileSystem = FileSystem.SYSTEM),
            mimeType = null,
            dataSource = DataSource.NETWORK,
        )
    }

    class Factory(
        private val matrixClient: MatrixClient,
    ) : Fetcher.Factory<Uri> {
        override fun create(
            data: Uri,
            options: Options,
            imageLoader: ImageLoader,
        ): Fetcher? {
            val url = data.toString()
            if (!url.startsWith("mxc://")) return null
            return MxcMediaFetcher(matrixClient, url)
        }
    }
}

private object MxcMediaCache {
    private val lock = Any()
    private val cache =
        object : LinkedHashMap<String, ByteArray>(IN_MEMORY_MXC_CACHE_MAX_ENTRIES, 0.75f, true) {
            override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, ByteArray>?): Boolean =
                size > IN_MEMORY_MXC_CACHE_MAX_ENTRIES
        }

    fun get(url: String): ByteArray? = synchronized(lock) { cache[url] }

    fun put(
        url: String,
        bytes: ByteArray,
    ) {
        synchronized(lock) {
            cache[url] = bytes
        }
    }
}
