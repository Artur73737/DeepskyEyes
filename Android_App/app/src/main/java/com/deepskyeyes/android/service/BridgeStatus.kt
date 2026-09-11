package com.deepskyeyes.android.service

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class BridgeSnapshot(
    val running: Boolean = false, val connected: Boolean = false,
    val cameraState: String = "CLOSED", val captured: Long = 0,
    val rxBytes: Long = 0, val txBytes: Long = 0,
    val thermal: String = "Unavailable", val dump: String = "",
    val capabilities: String = "[]", val busy: Boolean = false,
    val preview: android.graphics.Bitmap? = null, val lastFile: String? = null,
    val logs: List<String> = emptyList(),
)
object BridgeStatus {
    private val mutable = MutableStateFlow(BridgeSnapshot())
    val state = mutable.asStateFlow()
    fun update(change: (BridgeSnapshot) -> BridgeSnapshot) = mutable.update(change)
    fun log(message: String) = update { it.copy(logs = (it.logs + message).takeLast(80)) }
}
