package com.poziomki.app

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.window.ComposeUIViewController
import platform.UIKit.UIColor

@OptIn(ExperimentalComposeUiApi::class)
@Suppress("ktlint:standard:function-naming")
fun MainViewController() =
    ComposeUIViewController(configure = { opaque = false }) {
        App()
    }.apply {
        // Compose's Skia surface defaults to opaque with a white clear color,
        // which bleeds through during NavHost slide transitions when the
        // incoming screen fades in from low alpha. Marking the surface
        // non-opaque lets the black UIView background show instead.
        view.backgroundColor = UIColor.blackColor
        view.setOpaque(false)
    }
