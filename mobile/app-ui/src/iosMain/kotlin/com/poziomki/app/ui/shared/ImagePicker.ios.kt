package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.useContents
import kotlinx.cinterop.usePinned
import org.jetbrains.skia.Image
import platform.CoreGraphics.CGRectMake
import platform.CoreGraphics.CGSizeMake
import platform.Foundation.NSData
import platform.Foundation.NSItemProvider
import platform.PhotosUI.PHPickerConfiguration
import platform.PhotosUI.PHPickerFilter
import platform.PhotosUI.PHPickerResult
import platform.PhotosUI.PHPickerViewController
import platform.PhotosUI.PHPickerViewControllerDelegateProtocol
import platform.UIKit.UIApplication
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
import platform.UIKit.UISceneActivationStateForegroundActive
import platform.UIKit.UIViewController
import platform.UIKit.UIWindow
import platform.UIKit.UIWindowScene
import platform.darwin.NSObject
import platform.darwin.dispatch_async
import platform.darwin.dispatch_get_main_queue
import platform.posix.memcpy

private const val MAX_DIMENSION = 1920.0
private const val MAX_BYTES = 700 * 1024
private const val START_QUALITY = 0.85
private const val MIN_QUALITY = 0.5
private const val QUALITY_STEP = 0.1
private const val IMAGE_UTI = "public.image"

@Composable
actual fun rememberCameraCapture(onResult: (ByteArray?) -> Unit): () -> Unit {
    val coordinator = remember { CameraCoordinator() }
    DisposableEffect(Unit) { onDispose { coordinator.onResult = null } }
    return {
        coordinator.onResult = onResult
        presentCamera(coordinator)
    }
}

@Composable
actual fun rememberSingleImagePicker(onResult: (ByteArray?) -> Unit): () -> Unit {
    val coordinator = remember { PhotoPickerCoordinator() }
    DisposableEffect(Unit) { onDispose { coordinator.onComplete = null } }
    return {
        coordinator.onComplete = { images -> onResult(images.firstOrNull()) }
        presentPhotoPicker(coordinator, selectionLimit = 1)
    }
}

@Composable
actual fun rememberMultiImagePicker(onResult: (List<ByteArray>) -> Unit): () -> Unit {
    val coordinator = remember { PhotoPickerCoordinator() }
    DisposableEffect(Unit) { onDispose { coordinator.onComplete = null } }
    return {
        coordinator.onComplete = onResult
        presentPhotoPicker(coordinator, selectionLimit = 0)
    }
}

@Composable
actual fun rememberSingleFilePicker(onResult: (PickedFile?) -> Unit): () -> Unit = { /* not used on iOS */ }

actual fun decodeImageBytes(bytes: ByteArray): ImageBitmap? =
    try {
        Image.makeFromEncoded(bytes).toComposeImageBitmap()
    } catch (_: Throwable) {
        null
    }

// --- PHPicker (photo library) ---

private class PhotoPickerCoordinator :
    NSObject(),
    PHPickerViewControllerDelegateProtocol {
    var onComplete: ((List<ByteArray>) -> Unit)? = null

    override fun picker(
        picker: PHPickerViewController,
        didFinishPicking: List<*>,
    ) {
        picker.dismissViewControllerAnimated(flag = true, completion = null)
        val callback = onComplete ?: return
        val providers = didFinishPicking.mapNotNull { (it as? PHPickerResult)?.itemProvider }
        loadImages(providers, callback)
    }
}

@OptIn(ExperimentalForeignApi::class)
private fun presentPhotoPicker(
    coordinator: PhotoPickerCoordinator,
    selectionLimit: Int,
) {
    val config =
        PHPickerConfiguration().apply {
            this.selectionLimit = selectionLimit.toLong()
            this.filter = PHPickerFilter.imagesFilter()
        }
    val picker = PHPickerViewController(configuration = config)
    picker.delegate = coordinator
    rootViewController()?.presentViewController(picker, animated = true, completion = null)
}

private fun loadImages(
    providers: List<NSItemProvider>,
    onComplete: (List<ByteArray>) -> Unit,
) {
    if (providers.isEmpty()) {
        onComplete(emptyList())
        return
    }
    val collected = mutableListOf<ByteArray>()
    var remaining = providers.size
    providers.forEach { provider ->
        provider.loadDataRepresentationForTypeIdentifier(IMAGE_UTI) { data, _ ->
            // Completion runs off the main thread; hop to main for UIKit + serialized accumulation.
            dispatch_async(dispatch_get_main_queue()) {
                data?.let { encodeFromData(it) }?.let(collected::add)
                remaining -= 1
                if (remaining == 0) onComplete(collected.toList())
            }
        }
    }
}

// --- Camera ---

private class CameraCoordinator :
    NSObject(),
    UIImagePickerControllerDelegateProtocol,
    UINavigationControllerDelegateProtocol {
    var onResult: ((ByteArray?) -> Unit)? = null

    override fun imagePickerController(
        picker: UIImagePickerController,
        didFinishPickingMediaWithInfo: Map<Any?, *>,
    ) {
        picker.dismissViewControllerAnimated(flag = true, completion = null)
        val callback = onResult
        onResult = null
        val image = didFinishPickingMediaWithInfo[UIImagePickerControllerOriginalImage] as? UIImage
        callback?.invoke(image?.let(::encodeImage))
    }

    override fun imagePickerControllerDidCancel(picker: UIImagePickerController) {
        picker.dismissViewControllerAnimated(flag = true, completion = null)
        val callback = onResult
        onResult = null
        callback?.invoke(null)
    }
}

private fun presentCamera(coordinator: CameraCoordinator) {
    if (!UIImagePickerController.isSourceTypeAvailable(
            UIImagePickerControllerSourceType.UIImagePickerControllerSourceTypeCamera,
        )
    ) {
        coordinator.onResult?.invoke(null)
        coordinator.onResult = null
        return
    }
    val picker =
        UIImagePickerController().apply {
            sourceType = UIImagePickerControllerSourceType.UIImagePickerControllerSourceTypeCamera
            delegate = coordinator
        }
    rootViewController()?.presentViewController(picker, animated = true, completion = null)
}

// --- Helpers ---

private fun rootViewController(): UIViewController? {
    val app = UIApplication.sharedApplication
    // keyWindow is deprecated and returns nil on iPad / multi-scene apps, which
    // silently dropped the picker presentation. Resolve via the active window scene.
    val scenes = app.connectedScenes.mapNotNull { it as? UIWindowScene }
    val scene =
        scenes.firstOrNull { it.activationState == UISceneActivationStateForegroundActive }
            ?: scenes.firstOrNull()
    val window =
        scene?.keyWindow
            ?: scene?.windows?.mapNotNull { it as? UIWindow }?.firstOrNull()
            ?: app.keyWindow
    return window?.rootViewController
}

private fun encodeFromData(data: NSData): ByteArray? = UIImage(data = data)?.let(::encodeImage)

@OptIn(ExperimentalForeignApi::class)
private fun encodeImage(image: UIImage): ByteArray? {
    val resized = resizeImage(image, MAX_DIMENSION)
    var quality = START_QUALITY
    var data = UIImageJPEGRepresentation(resized, quality)
    while (data != null && data.length.toInt() > MAX_BYTES && quality > MIN_QUALITY) {
        quality -= QUALITY_STEP
        data = UIImageJPEGRepresentation(resized, quality)
    }
    return data?.toByteArray()
}

@OptIn(ExperimentalForeignApi::class)
private fun resizeImage(
    image: UIImage,
    maxDimension: Double,
): UIImage {
    val size = image.size.useContents { width to height }
    val maxSide = maxOf(size.first, size.second)
    if (maxSide <= maxDimension || maxSide == 0.0) return image
    val scale = maxDimension / maxSide
    val newWidth = size.first * scale
    val newHeight = size.second * scale
    UIGraphicsBeginImageContextWithOptions(CGSizeMake(newWidth, newHeight), false, 1.0)
    image.drawInRect(CGRectMake(0.0, 0.0, newWidth, newHeight))
    val result = UIGraphicsGetImageFromCurrentImageContext()
    UIGraphicsEndImageContext()
    return result ?: image
}

@OptIn(ExperimentalForeignApi::class)
private fun NSData.toByteArray(): ByteArray {
    val size = length.toInt()
    if (size == 0) return ByteArray(0)
    val out = ByteArray(size)
    out.usePinned { pinned ->
        memcpy(pinned.addressOf(0), bytes, length)
    }
    return out
}
