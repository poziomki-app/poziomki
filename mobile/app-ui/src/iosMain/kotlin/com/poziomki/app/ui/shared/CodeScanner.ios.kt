@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class, ExperimentalObjCName::class)

package com.poziomki.app.ui.shared

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import platform.AVFoundation.AVCaptureConnection
import platform.AVFoundation.AVCaptureDevice
import platform.AVFoundation.AVCaptureDeviceInput
import platform.AVFoundation.AVCaptureMetadataOutput
import platform.AVFoundation.AVCaptureMetadataOutputObjectsDelegateProtocol
import platform.AVFoundation.AVCaptureOutput
import platform.AVFoundation.AVCaptureSession
import platform.AVFoundation.AVCaptureSessionPresetHigh
import platform.AVFoundation.AVCaptureVideoPreviewLayer
import platform.AVFoundation.AVLayerVideoGravityResizeAspectFill
import platform.AVFoundation.AVMediaTypeVideo
import platform.AVFoundation.AVMetadataMachineReadableCodeObject
import platform.AVFoundation.AVMetadataObjectTypeQRCode
import platform.CoreGraphics.CGRectMake
import platform.UIKit.UIApplication
import platform.UIKit.UIButton
import platform.UIKit.UIButtonTypeSystem
import platform.UIKit.UIColor
import platform.UIKit.UIControlEventTouchUpInside
import platform.UIKit.UIControlStateNormal
import platform.UIKit.UIModalPresentationFullScreen
import platform.UIKit.UIViewController
import platform.darwin.dispatch_async
import platform.darwin.dispatch_get_main_queue
import kotlin.experimental.ExperimentalObjCName
import kotlin.native.ObjCName

@Composable
actual fun rememberCodeScanner(onResult: (String?) -> Unit): () -> Unit {
    val callback = remember(onResult) { onResult }
    return {
        val rootVC =
            UIApplication.sharedApplication.keyWindow?.rootViewController?.let {
                var vc: UIViewController? = it
                while (vc?.presentedViewController != null) vc = vc.presentedViewController
                vc
            }
        if (rootVC == null) {
            callback(null)
        } else {
            val scanner = QrScannerViewController(callback)
            scanner.modalPresentationStyle = UIModalPresentationFullScreen
            rootVC.presentViewController(scanner, animated = true, completion = null)
        }
    }
}

@OptIn(ExperimentalForeignApi::class, BetaInteropApi::class, ExperimentalObjCName::class)
private class QrScannerViewController(
    private val onResult: (String?) -> Unit,
) : UIViewController(nibName = null, bundle = null),
    AVCaptureMetadataOutputObjectsDelegateProtocol {
    private var captureSession: AVCaptureSession? = null
    private var resolved = false

    override fun viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = UIColor.blackColor

        val device = AVCaptureDevice.defaultDeviceWithMediaType(AVMediaTypeVideo)
        if (device == null) {
            finishWith(null)
            return
        }
        val session = AVCaptureSession()
        session.sessionPreset = AVCaptureSessionPresetHigh

        val input = AVCaptureDeviceInput.deviceInputWithDevice(device, error = null)
        if (input == null || !session.canAddInput(input)) {
            finishWith(null)
            return
        }
        session.addInput(input)

        val output = AVCaptureMetadataOutput()
        if (!session.canAddOutput(output)) {
            finishWith(null)
            return
        }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(this, queue = dispatch_get_main_queue())
        output.metadataObjectTypes = listOf(AVMetadataObjectTypeQRCode)

        val preview = AVCaptureVideoPreviewLayer(session = session)
        preview.frame = view.bounds
        preview.videoGravity = AVLayerVideoGravityResizeAspectFill
        view.layer.addSublayer(preview)

        addCancelButton()
        session.startRunning()
        captureSession = session
    }

    private fun addCancelButton() {
        val cancel = UIButton.buttonWithType(UIButtonTypeSystem)
        cancel.setTitle("Anuluj", forState = UIControlStateNormal)
        cancel.setTitleColor(UIColor.whiteColor, forState = UIControlStateNormal)
        cancel.setFrame(CGRectMake(16.0, 56.0, 80.0, 36.0))
        cancel.setBackgroundColor(UIColor.darkGrayColor.colorWithAlphaComponent(0.6))
        cancel.layer.setCornerRadius(8.0)
        cancel.addTarget(
            target = this,
            action = platform.objc.sel_registerName("cancelTapped"),
            forControlEvents = UIControlEventTouchUpInside,
        )
        view.addSubview(cancel)
    }

    @ObjCName("cancelTapped")
    fun cancelTapped() {
        finishWith(null)
    }

    override fun captureOutput(
        output: AVCaptureOutput,
        didOutputMetadataObjects: List<*>,
        fromConnection: AVCaptureConnection,
    ) {
        if (resolved) return
        val first = didOutputMetadataObjects.firstOrNull() as? AVMetadataMachineReadableCodeObject
        val code = first?.stringValue ?: return
        finishWith(code)
    }

    private fun finishWith(value: String?) {
        if (resolved) return
        resolved = true
        captureSession?.stopRunning()
        dispatch_async(dispatch_get_main_queue()) {
            dismissViewControllerAnimated(true) {
                onResult(value)
            }
        }
    }
}
