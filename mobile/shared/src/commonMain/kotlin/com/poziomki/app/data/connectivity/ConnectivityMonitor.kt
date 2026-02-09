package com.poziomki.app.data.connectivity

import kotlinx.coroutines.flow.StateFlow

interface ConnectivityMonitor {
    val isOnline: StateFlow<Boolean>
}
