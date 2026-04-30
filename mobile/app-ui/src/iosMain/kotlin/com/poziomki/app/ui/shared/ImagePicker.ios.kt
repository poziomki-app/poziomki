@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import platform.CoreGraphics.CGImageGetHeight
import platform.CoreGraphics.CGImageGetWidth
import platform.CoreGraphics.CGRectMake
import platform.CoreGraphics.CGSizeMake
import platform.Foundation.NSData
import platform.Foundation.NSError
import platform.Foundation.NSItemProvider
import platform.Foundation.NSURL
import platform.Foundation.create
import platform.Foundation.dataWithContentsOfURL
import platform.PhotosUI.PHPickerConfiguration
import platform.PhotosUI.PHPickerFilter
import platform.PhotosUI.PHPickerResult
import platform.PhotosUI.PHPickerViewController
import platform.PhotosUI.PHPickerViewControllerDelegateProtocol
import platform.UIKit.UIApplication
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerMode
import platform.UIKit.UIDocumentPickerViewController
import platform.UIKit.UIGraphicsBeginImageContextWithOptions
import platform.UIKit.UIGraphicsEndImageContext
import platform.UIKit.UIGraphicsGetImageFromCurrentImageContext
import platform.UIKit.UIImage
import platform.UIKit.UIImageJPEGRepresentation
import platform.UIKit.UIImagePickerController
import platform.UIKit.UIImagePickerControllerDelegateProtocol
import platform.UIKit.UIImagePickerControllerOriginalImage
import platform.UIKit.UIImagePickerControllerSourceType
import platform.UIKit.UINavigationControllerDelegateProtocol
import platform.UIKit.UIViewController
import platform.darwin.NSObject
import platform.darwin.dispatch_async
import platform.darwin.dispatch_get_main_queue
import platform.posix.memcpy
import org.jetbrains.skia.Image as SkiaImage

private const val MAX_DIMENSION: Double = 1920.0
private const val DEFAULT_JPEG_QUALITY: Double = 0.85
private const val MIN_JPEG_QUALITY: Double = 0.50
private const val QUALITY_STEP: Double = 0.10
private const val MAX_BYTES: Int = 700 * 1024
private const val MULTI_PICKER_LIMIT: Long = 6
private const val IMAGE_UTI: String = "public.image"
private const val FILE_UTI: String = "public.item"

private fun rootViewController(): UIViewController? {
    var vc: UIViewController? = UIApplication.sharedApplication.keyWindow?.rootViewController
    while (vc?.presentedViewController != null) vc = vc.presentedViewController
    return vc
}

private fun ByteArray.toNSData(): NSData =
    if (isEmpty()) {
        NSData()
    } else {
        usePinned { pinned ->
            NSData.create(bytes = pinned.addressOf(0), length = size.toULong())
        }
    }

private fun NSData.toByteArray(): ByteArray {
    val len = length.toInt()
    if (len == 0) return ByteArray(0)
    val out = ByteArray(len)
    out.usePinned { pinned ->
        memcpy(pinned.addressOf(0), bytes, length)
    }
    return out
}

/** UIImage pixel size (via the underlying CGImage) — sidesteps CGSize struct interop. */
private fun pixelSize(image: UIImage): Pair<Double, Double> {
    val cg = image.CGImage ?: return 0.0 to 0.0
    return CGImageGetWidth(cg).toDouble() to CGImageGetHeight(cg).toDouble()
}

private fun scaleDown(
    image: UIImage,
    maxDimension: Double,
): UIImage {
    val (w, h) = pixelSize(image)
    val largest = if (w > h) w else h
    if (largest <= maxDimension || largest <= 0.0) return image
    val factor = maxDimension / largest
    return scaleTo(image, w * factor, h * factor) ?: image
}

private fun scaleTo(
    image: UIImage,
    width: Double,
    height: Double,
): UIImage? {
    if (width <= 0.0 || height <= 0.0) return null
    UIGraphicsBeginImageContextWithOptions(CGSizeMake(width, height), false, 1.0)
    image.drawInRect(CGRectMake(0.0, 0.0, width, height))
    val result = UIGraphicsGetImageFromCurrentImageContext()
    UIGraphicsEndImageContext()
    return result
}

/** Compress a UIImage to JPEG, mirroring Android's quality-then-rescale loop. */
private fun compressUIImage(image: UIImage): ByteArray? {
    val resized = scaleDown(image, MAX_DIMENSION)
    var quality = DEFAULT_JPEG_QUALITY
    var data: NSData = UIImageJPEGRepresentation(resized, quality) ?: return null

    while (data.length.toInt() > MAX_BYTES && quality > MIN_JPEG_QUALITY) {
        quality -= QUALITY_STEP
        data = UIImageJPEGRepresentation(resized, quality) ?: return null
    }

    if (data.length.toInt() > MAX_BYTES) {
        val (w, h) = pixelSize(resized)
        val factor = kotlin.math.sqrt(MAX_BYTES.toDouble() / data.length.toDouble())
        val smaller = scaleTo(resized, w * factor, h * factor)
        if (smaller != null) {
            val smallerData = UIImageJPEGRepresentation(smaller, quality)
            if (smallerData != null) data = smallerData
        }
    }
    return data.toByteArray()
}

actual fun decodeImageBytes(bytes: ByteArray): ImageBitmap? =
    runCatching { SkiaImage.makeFromEncoded(bytes).toComposeImageBitmap() }.getOrNull()

// ---------------------------------------------------------------------------
// Camera (UIImagePickerController)
// ---------------------------------------------------------------------------

private class CameraDelegate(
    val onResult: (ByteArray?) -> Unit,
) : NSObject(),
    UIImagePickerControllerDelegateProtocol,
    UINavigationControllerDelegateProtocol {
    /** Strong self-reference released when the picker dismisses. */
    var retained: CameraDelegate? = null

    override fun imagePickerController(
        picker: UIImagePickerController,
        didFinishPickingMediaWithInfo: Map<Any?, *>,
    ) {
        val image = didFinishPickingMediaWithInfo[UIImagePickerControllerOriginalImage] as? UIImage
        picker.dismissViewControllerAnimated(true) {
            val bytes = image?.let { compressUIImage(it) }
            onResult(bytes)
            retained = null
        }
    }

    override fun imagePickerControllerDidCancel(picker: UIImagePickerController) {
        picker.dismissViewControllerAnimated(true) {
            onResult(null)
            retained = null
        }
    }
}

@Composable
actual fun rememberCameraCapture(onResult: (ByteArray?) -> Unit): () -> Unit {
    val callback = remember(onResult) { onResult }
    return {
        val cameraType = UIImagePickerControllerSourceType.UIImagePickerControllerSourceTypeCamera
        if (!UIImagePickerController.isSourceTypeAvailable(cameraType)) {
            callback(null)
        } else {
            val delegate = CameraDelegate(callback)
            delegate.retained = delegate
            val picker =
                UIImagePickerController().apply {
                    sourceType = cameraType
                    this.delegate = delegate
                }
            val root = rootViewController()
            if (root == null) {
                delegate.retained = null
                callback(null)
            } else {
                root.presentViewController(picker, animated = true, completion = null)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Gallery (PHPickerViewController, single + multi)
// ---------------------------------------------------------------------------

private class GalleryDelegate(
    val multi: Boolean,
    val onSingle: ((ByteArray?) -> Unit)? = null,
    val onMulti: ((List<ByteArray>) -> Unit)? = null,
) : NSObject(),
    PHPickerViewControllerDelegateProtocol {
    var retained: GalleryDelegate? = null

    override fun picker(
        picker: PHPickerViewController,
        didFinishPicking: List<*>,
    ) {
        @Suppress("UNCHECKED_CAST")
        val results = didFinishPicking as List<PHPickerResult>
        picker.dismissViewControllerAnimated(true) {
            if (results.isEmpty()) {
                if (multi) onMulti?.invoke(emptyList()) else onSingle?.invoke(null)
                retained = null
                return@dismissViewControllerAnimated
            }
            // PHPicker's loadDataRepresentation completion is invoked on a
            // private background queue with no ordering guarantees, so we
            // must hop back to main before mutating shared state or calling
            // back into Compose.
            val collected = mutableListOf<ByteArray>()
            var pending = results.size
            val finish: () -> Unit = {
                if (multi) {
                    onMulti?.invoke(collected.toList())
                } else {
                    onSingle?.invoke(collected.firstOrNull())
                }
                retained = null
            }
            results.forEach { result ->
                val provider: NSItemProvider = result.itemProvider
                provider.loadDataRepresentationForTypeIdentifier(IMAGE_UTI) { data: NSData?, _: NSError? ->
                    val bytes =
                        data?.toByteArray()?.let { raw ->
                            UIImage.imageWithData(raw.toNSData())?.let { compressUIImage(it) }
                        }
                    dispatch_async(dispatch_get_main_queue()) {
                        if (bytes != null) collected.add(bytes)
                        pending -= 1
                        if (pending == 0) finish()
                    }
                }
            }
        }
    }
}

private fun presentPhPicker(
    multi: Boolean,
    delegate: GalleryDelegate,
) {
    val config =
        PHPickerConfiguration().apply {
            selectionLimit = if (multi) MULTI_PICKER_LIMIT else 1L
            filter = PHPickerFilter.imagesFilter
        }
    val picker =
        PHPickerViewController(configuration = config).apply {
            this.delegate = delegate
        }
    val root = rootViewController()
    if (root == null) {
        delegate.retained = null
        if (multi) delegate.onMulti?.invoke(emptyList()) else delegate.onSingle?.invoke(null)
    } else {
        root.presentViewController(picker, animated = true, completion = null)
    }
}

@Composable
actual fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit {
    val callback = remember(onResult) { onResult }
    return {
        val delegate = GalleryDelegate(multi = false, onSingle = callback)
        delegate.retained = delegate
        presentPhPicker(false, delegate)
    }
}

@Composable
actual fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit {
    val callback = remember(onResult) { onResult }
    return {
        val delegate = GalleryDelegate(multi = true, onMulti = callback)
        delegate.retained = delegate
        presentPhPicker(true, delegate)
    }
}

// ---------------------------------------------------------------------------
// File picker (UIDocumentPickerViewController)
// ---------------------------------------------------------------------------

private class FilePickerDelegate(
    val onResult: (PickedFile?) -> Unit,
) : NSObject(),
    UIDocumentPickerDelegateProtocol {
    var retained: FilePickerDelegate? = null

    override fun documentPicker(
        controller: UIDocumentPickerViewController,
        didPickDocumentsAtURLs: List<*>,
    ) {
        @Suppress("UNCHECKED_CAST")
        val urls = didPickDocumentsAtURLs as List<NSURL>
        val url = urls.firstOrNull()
        if (url == null) {
            onResult(null)
            retained = null
            return
        }
        val accessed = url.startAccessingSecurityScopedResource()
        val data = NSData.dataWithContentsOfURL(url)
        if (accessed) url.stopAccessingSecurityScopedResource()
        val bytes = data?.toByteArray()
        if (bytes == null) {
            onResult(null)
        } else {
            val name = url.lastPathComponent ?: "attachment"
            val mime = url.pathExtension?.let { mimeForExtension(it) }
            onResult(PickedFile(name = name, bytes = bytes, mimeType = mime))
        }
        retained = null
    }

    override fun documentPickerWasCancelled(controller: UIDocumentPickerViewController) {
        onResult(null)
        retained = null
    }
}

private fun mimeForExtension(ext: String): String? =
    when (ext.lowercase()) {
        "jpg", "jpeg" -> "image/jpeg"
        "png" -> "image/png"
        "gif" -> "image/gif"
        "webp" -> "image/webp"
        "pdf" -> "application/pdf"
        "txt" -> "text/plain"
        "json" -> "application/json"
        "zip" -> "application/zip"
        else -> null
    }

@Composable
actual fun rememberSingleFilePicker(onResult: (PickedFile?) -> Unit): () -> Unit {
    val callback = remember(onResult) { onResult }
    return {
        val delegate = FilePickerDelegate(callback)
        delegate.retained = delegate
        val picker =
            UIDocumentPickerViewController(
                documentTypes = listOf(FILE_UTI),
                inMode = UIDocumentPickerMode.UIDocumentPickerModeImport,
            ).apply {
                this.delegate = delegate
                allowsMultipleSelection = false
            }
        val root = rootViewController()
        if (root == null) {
            delegate.retained = null
            callback(null)
        } else {
            root.presentViewController(picker, animated = true, completion = null)
        }
    }
}
