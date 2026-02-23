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

class MxcMediaFetcher(
    private val matrixClient: MatrixClient,
    private val mxcUrl: String,
) : Fetcher {
    override suspend fun fetch(): FetchResult {
        withTimeoutOrNull(CLIENT_READY_TIMEOUT_MS) {
            matrixClient.state.first { it is MatrixClientState.Ready }
        } ?: throw IOException("Matrix client not ready for media fetch")

        val bytes =
            matrixClient.getMediaThumbnail(mxcUrl, THUMBNAIL_SIZE, THUMBNAIL_SIZE)
                ?: matrixClient.getMediaContent(mxcUrl)
                ?: throw IOException("Matrix media not available for $mxcUrl")
        if (bytes.isEmpty()) throw IOException("Matrix media empty for $mxcUrl")
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
