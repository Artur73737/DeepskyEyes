package com.deepskyeyes.android

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.deepskyeyes.android.service.BridgeStatus
import com.deepskyeyes.android.service.CameraService
import com.deepskyeyes.android.ui.CameraPanel

class MainActivity : ComponentActivity() {
    private val exportCapture = registerForActivityResult(ActivityResultContracts.CreateDocument("application/octet-stream")) { uri ->
        val file = BridgeStatus.state.value.lastFile
        if(uri != null && file != null) Thread {
            runCatching { contentResolver.openOutputStream(uri)?.use { output -> java.io.File(file).inputStream().use { it.copyTo(output) } } ?: error("Cannot open destination") }
                .onSuccess { BridgeStatus.log("Scatto esportato") }.onFailure { BridgeStatus.log("Export: ${it.message}") }
        }.start()
    }
    private val permissions = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        if(checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) startBridge()
        else BridgeStatus.log("Camera permission is required. Enable it in app settings.")
    }
    private val export = registerForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { uri ->
        if(uri != null) {
            val dump = BridgeStatus.state.value.dump
            Thread {
                runCatching { contentResolver.openOutputStream(uri)?.use { it.write(dump.toByteArray()) } ?: error("Cannot open document") }
                    .onSuccess { BridgeStatus.log("Capability dump exported") }
                    .onFailure { BridgeStatus.log("Export failed: ${it.message}") }
            }.start()
        }
    }
    private fun startBridge() {
        runCatching { startForegroundService(Intent(this,CameraService::class.java)) }
            .onFailure { BridgeStatus.log("Cannot start service: ${it.message}") }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val state by BridgeStatus.state.collectAsStateWithLifecycle()
            var tab by remember { mutableIntStateOf(0) }
            val colors = darkColorScheme(primary=Color(0xFF94DBBC),background=Color(0xFF0B1017),surface=Color(0xFF151E29),secondary=Color(0xFFAEC7EA))
            MaterialTheme(colorScheme = colors) {
                Surface(Modifier.fillMaxSize()) {
                    Column(Modifier.safeDrawingPadding().verticalScroll(rememberScrollState()).padding(24.dp),verticalArrangement=Arrangement.spacedBy(18.dp)) {
                        Text("DEEPSKYEYES",style=MaterialTheme.typography.labelLarge,color=colors.primary)
                        Text("Camera station",style=MaterialTheme.typography.headlineLarge)
                        Text("Android 17 · Camera2 · RAW",color=colors.secondary)
                        Row(horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                            listOf("Connessione","Camera","Diagnostica").forEachIndexed { index,title ->
                                FilterChip(tab==index,{tab=index},label={Text(title)})
                            }
                        }
                        if(tab == 1) CameraPanel(state,onAction={ operation,request ->
                            startService(Intent(this@MainActivity,CameraService::class.java).setAction(CameraService.LOCAL).putExtra("operation",operation).putExtra("request",request))
                        },onExport={exportCapture.launch(state.lastFile?.substringAfterLast('/') ?: "capture.dng")})
                        if(tab != 1) {
                        Card(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(20.dp),verticalArrangement=Arrangement.spacedBy(8.dp)) {
                                Text(if(state.connected) "Desktop connected" else if(state.running) "Waiting for desktop" else "Ready to connect",style=MaterialTheme.typography.titleLarge)
                                Text(state.cameraState,fontFamily=FontFamily.Monospace)
                                Text("Frames transferred: ${state.captured}")
                                Text("RX ${state.rxBytes / 1024} KiB   ·   TX ${state.txBytes / 1024} KiB")
                                Text("Thermal: ${state.thermal}")
                            }
                        }
                        Row(horizontalArrangement=Arrangement.spacedBy(12.dp)) {
                            Button(onClick={ permissions.launch(arrayOf(Manifest.permission.CAMERA,Manifest.permission.POST_NOTIFICATIONS)) },enabled=!state.running) { Text("Start service") }
                            OutlinedButton(onClick={ stopService(Intent(this@MainActivity,CameraService::class.java)) },enabled=state.running) { Text("Stop") }
                        }
                        Text("USB connection",style=MaterialTheme.typography.titleMedium)
                        Text("Enable USB debugging, authorize this computer and start the Rust desktop app with ADB. Keep this service active during acquisition.")
                        Text("127.0.0.1:7878  ·  Protocol v1",fontFamily=FontFamily.Monospace,color=colors.primary)
                        Row(horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                            OutlinedButton(onClick={ export.launch("deepsky-capabilities.json") },enabled=state.dump.isNotEmpty()) { Text("Export capabilities") }
                            TextButton(onClick={ startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS,android.net.Uri.parse("package:$packageName"))) }) { Text("Permissions") }
                        }
                        Text("Diagnostics",style=MaterialTheme.typography.titleMedium)
                        if(state.logs.isEmpty()) Text("Start the service to enumerate the device's real camera capabilities.")
                        state.logs.takeLast(20).reversed().forEach { line ->
                            Text(line,style=MaterialTheme.typography.bodySmall,fontFamily=FontFamily.Monospace,modifier=Modifier.fillMaxWidth().background(colors.surface).padding(8.dp))
                        }
                        Text("Discover. Validate. Capture. Record everything.",color=colors.secondary,style=MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
    }
}
