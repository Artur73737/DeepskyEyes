package com.deepskyeyes.android.service

import android.Manifest
import android.app.*
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.os.*
import com.deepskyeyes.android.MainActivity
import com.deepskyeyes.android.camera.CameraEngine
import com.deepskyeyes.android.protocol.obj
import com.deepskyeyes.android.transport.BridgeServer
import java.util.concurrent.Executors

class CameraService : Service() {
    private var engine: CameraEngine? = null
    private var server: BridgeServer? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private val worker = Executors.newSingleThreadExecutor { Thread(it,"Deepsky-bridge") }
    private val localWorker = Executors.newSingleThreadExecutor { Thread(it,"Deepsky-local") }
    override fun onBind(intent: Intent?) = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if(intent?.action == STOP) { stopSelf(); return START_NOT_STICKY }
        if(intent?.action == LOCAL) {
            val camera = engine
            if(camera != null && !BridgeStatus.state.value.connected && !BridgeStatus.state.value.busy) {
                BridgeStatus.update { it.copy(busy = true) }
                val request = intent.getStringExtra("request")
                val operation = intent.getStringExtra("operation") ?: "preview"
                localWorker.execute {
                    try {
                        synchronized(camera) { LocalCamera.execute(this,camera,operation,request) }
                    } catch(e: Exception) { BridgeStatus.log("Local camera: ${e.message}") }
                    finally { BridgeStatus.update { it.copy(busy = false) } }
                }
            }
            return START_NOT_STICKY
        }
        if(server != null) return START_NOT_STICKY
        if(checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) { stopSelf(); return START_NOT_STICKY }
        val notifications = getSystemService(NotificationManager::class.java)
        notifications.createNotificationChannel(NotificationChannel(CHANNEL,"Camera connection",NotificationManager.IMPORTANCE_LOW))
        val launch = PendingIntent.getActivity(this,0,Intent(this,MainActivity::class.java),PendingIntent.FLAG_IMMUTABLE)
        val stop = PendingIntent.getService(this,1,Intent(this,CameraService::class.java).setAction(STOP),PendingIntent.FLAG_IMMUTABLE)
        val notification = Notification.Builder(this,CHANNEL).setSmallIcon(android.R.drawable.ic_menu_camera)
            .setContentTitle("DeepskyEyes camera service").setContentText("Ready for desktop over USB / ADB")
            .setContentIntent(launch).setOngoing(true).addAction(Notification.Action.Builder(null,"Stop",stop).build()).build()
        try {
            startForeground(1,notification,ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA)
            val power = getSystemService(PowerManager::class.java)
            wakeLock = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK,"DeepskyEyes:Camera").apply { setReferenceCounted(false); acquire() }
            val camera = CameraEngine(this) { state -> BridgeStatus.update { it.copy(cameraState = state) } }
            engine = camera
            val bridge = BridgeServer(camera) {
                val level = power.currentThermalStatus
                val severity = when {
                    level >= PowerManager.THERMAL_STATUS_SEVERE -> "critical"
                    level >= PowerManager.THERMAL_STATUS_MODERATE -> "warning"
                    else -> "nominal"
                }
                BridgeStatus.update { it.copy(thermal = "$severity (Android $level)") }
                // Android thermal service doesn't expose actual sensor temperature to ordinary apps.
                obj("severity" to severity,"temperature_c" to null,"origin" to "Device")
            }
            server = bridge
            worker.execute {
                try {
                    val dump = camera.discovery.dump().toString(2)
                    BridgeStatus.update { it.copy(dump = dump,capabilities = camera.discovery.discover().toString()) }
                    bridge.run()
                } catch(e: Exception) { BridgeStatus.log("Startup failed: ${e.message}") }
                finally { camera.close(); stopSelf() }
            }
        } catch(e: Exception) { BridgeStatus.log("Service failed: ${e.message}"); stopSelf() }
        // Never try to reopen a camera from a background restart without visible user action.
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        server?.close(); server = null
        worker.shutdown()
        localWorker.shutdown()
        wakeLock?.let { if(it.isHeld) it.release() }; wakeLock = null
        BridgeStatus.update { it.copy(running = false,connected = false) }
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }
    companion object {
        const val STOP = "com.deepskyeyes.android.STOP"
        const val LOCAL = "com.deepskyeyes.android.LOCAL"
        private const val CHANNEL = "camera-service"
    }
}
