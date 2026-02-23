package com.poziomki.app.connectivity

import kotlinx.coroutines.flow.StateFlow

interface ConnectivityMonitor {
    val isOnline: StateFlow<Boolean>
}
