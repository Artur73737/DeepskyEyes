package com.deepskyeyes.android.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import com.deepskyeyes.android.protocol.*
import com.deepskyeyes.android.service.BridgeSnapshot
import com.deepskyeyes.android.service.BridgeStatus
import org.json.JSONArray
import org.json.JSONObject

@Composable
fun CameraPanel(state: BridgeSnapshot, onAction: (String,String?) -> Unit, onExport: () -> Unit) {
    val cameras = remember(state.capabilities) { runCatching { JSONArray(state.capabilities).objects() }.getOrDefault(emptyList()) }
    if(!state.running) { Text("Avvia il servizio nella scheda Connessione per leggere le fotocamere."); return }
    if(cameras.isEmpty()) { LinearProgressIndicator(Modifier.fillMaxWidth()); Text("Lettura capacità Camera2…"); return }
    var cameraIndex by remember { mutableIntStateOf(0) }
    val c = cameras[cameraIndex.coerceIn(cameras.indices)]
    val id = c.getString("camera_id")
    Selector("Fotocamera",cameras.map { "${it.getString("camera_id")} · ${it.optString("lens_facing")} · ${if(it.optBoolean("raw")) "RAW" else "JPEG"}" },cameraIndex,!state.busy && !state.connected) { cameraIndex = it }
    key(id) {
        val streams = c.getJSONArray("streams").objects()
        var streamIndex by remember { mutableIntStateOf(0) }
        val exposureRange = c.optJSONObject("exposure_ns")
        val sensitivityRange = c.optJSONObject("sensitivity")
        var exposure by remember { mutableStateOf(((exposureRange?.getLong("min") ?: 1).coerceAtLeast(50_000_000).coerceAtMost(exposureRange?.getLong("max") ?: 50_000_000) / 1e9).toString()) }
        var iso by remember { mutableStateOf((sensitivityRange?.getLong("min") ?: 100).toString()) }
        var focus by remember { mutableFloatStateOf(0f) }
        var locked by remember { mutableStateOf(true) }
        var zoom by remember { mutableStateOf("1.0") }
        val wbModes = c.getJSONArray("wb_modes").strings()
        var wbIndex by remember { mutableIntStateOf(wbModes.indexOf("daylight").takeIf { it >= 0 } ?: 0) }
        val enabled = !state.busy && !state.connected && streams.isNotEmpty() && c.optBoolean("manual_sensor")
        if(state.connected) Text("Il desktop controlla la camera. Disconnettilo prima dei controlli locali.",color=MaterialTheme.colorScheme.secondary)
        if(!c.optBoolean("manual_sensor")) Text("Controlli manuali non annunciati da questa camera.",color=MaterialTheme.colorScheme.error)
        if(streams.isEmpty()) Text("Nessun output supportato dall'adattatore.")
        Selector("Formato e risoluzione",streams.map { "${it.getString("format")} · ${it.getInt("width")} × ${it.getInt("height")} · ${it.getString("pixel_mode")}" },streamIndex,enabled) { streamIndex=it }
        Row(horizontalArrangement=Arrangement.spacedBy(12.dp)) {
            OutlinedTextField(exposure,{ exposure=it },label={Text("Esposizione (s)")},singleLine=true,enabled=enabled,keyboardOptions=KeyboardOptions(keyboardType=KeyboardType.Decimal),modifier=Modifier.weight(1f))
            OutlinedTextField(iso,{ iso=it },label={Text("ISO sensore")},singleLine=true,enabled=enabled,keyboardOptions=KeyboardOptions(keyboardType=KeyboardType.Number),modifier=Modifier.weight(1f))
        }
        Text("Esposizione: ${exposureRange?.getLong("min")?.div(1e9)}–${exposureRange?.getLong("max")?.div(1e9)} s · ISO ${sensitivityRange?.getLong("min")}–${sensitivityRange?.getLong("max")}",style=MaterialTheme.typography.bodySmall)
        val focusMax = c.optJSONObject("focus_millidiopters")?.getLong("max")?.div(1000f)
        Text("Fuoco manuale · ${"%.3f".format(focus)} diottrie (0 = infinito)")
        if(focusMax != null && focusMax > 0f) Slider(focus,{focus=it},enabled=enabled,valueRange=0f..focusMax)
        Row {
            Checkbox(locked,{locked=it},enabled=enabled && c.optBoolean("focus_lock"))
            Text("Mantieni fuoco manuale",modifier=Modifier.padding(top=12.dp))
        }
        if(wbModes.isNotEmpty()) Selector("Bilanciamento del bianco",wbModes,wbIndex,enabled) { wbIndex=it }
        if(!c.isNull("zoom_x1000")) OutlinedTextField(zoom,{zoom=it},label={Text("Zoom ×")},singleLine=true,enabled=enabled,keyboardOptions=KeyboardOptions(keyboardType=KeyboardType.Decimal),modifier=Modifier.fillMaxWidth())
        var advanced by remember { mutableStateOf(false) }
        val processing = c.getJSONObject("processing_modes")
        val chosen = remember { mutableStateMapOf<String,String>().apply {
            processing.keys().forEach { name -> val modes=processing.getJSONArray(name).strings(); modes.firstOrNull()?.let { put(name,if("off" in modes) "off" else it) } }
        } }
        var ois by remember { mutableStateOf("off") }
        var eis by remember { mutableStateOf("off") }
        TextButton(onClick={advanced=!advanced}) { Text(if(advanced) "Nascondi controlli avanzati" else "Processing e stabilizzazione") }
        if(advanced) {
            processing.keys().forEach { name ->
                val modes=processing.getJSONArray(name).strings()
                if(modes.isNotEmpty()) Selector(name,modes,modes.indexOf(chosen[name]).coerceAtLeast(0),enabled) { chosen[name]=modes[it] }
            }
            val oisModes=c.getJSONArray("ois_modes").strings(); val eisModes=c.getJSONArray("eis_modes").strings()
            if(oisModes.isNotEmpty()) Selector("OIS",oisModes,oisModes.indexOf(ois).coerceAtLeast(0),enabled) {ois=oisModes[it]}
            if(eisModes.isNotEmpty()) Selector("EIS",eisModes,eisModes.indexOf(eis).coerceAtLeast(0),enabled) {eis=eisModes[it]}
        }
        fun send(operation: String) {
            try {
                val seconds=exposure.replace(',','.').toDouble()
                require(seconds.isFinite() && seconds>0 && seconds<1e9) {"Esposizione non valida"}
                val sensitivity=iso.toLong()
                val zoomValue=zoom.replace(',','.').toDouble()
                require(zoomValue.isFinite() && zoomValue>0 && zoomValue<1e6) {"Zoom non valido"}
                val settings=obj("exposure_ns" to (seconds*1e9).toLong(),"sensitivity" to sensitivity,
                    "frame_duration_ns" to null,"stream" to streams[streamIndex],
                    "focus" to if(focusMax!=null) obj("Manual" to obj("millidiopters" to (focus*1000).toLong(),"locked" to locked)) else null,
                    "white_balance" to wbModes.getOrNull(wbIndex)?.let { obj("Mode" to it) },
                    "zoom_x1000" to if(c.isNull("zoom_x1000")) null else (zoomValue*1000).toLong(),
                    "crop" to null,"processing" to JSONObject().apply {chosen.forEach { (k,v)->put(k,v) }},
                    "ois" to ois.takeIf { it in c.getJSONArray("ois_modes").strings() },
                    "eis" to eis.takeIf { it in c.getJSONArray("eis_modes").strings() })
                onAction(operation,obj("request_id" to 1,"selection" to obj("camera_id" to id,"physical_id" to null),"settings" to settings).toString())
            } catch(e: Exception) { BridgeStatus.log("Parametri: ${e.message}") }
        }
        Row(horizontalArrangement=Arrangement.spacedBy(12.dp)) {
            OutlinedButton(onClick={send("preview")},enabled=enabled) { Text("Preview") }
            Button(onClick={send("capture")},enabled=enabled) { Text("Scatta") }
            TextButton(onClick={onAction("close",null)},enabled=enabled) { Text("Chiudi") }
        }
        if(state.busy) { LinearProgressIndicator(Modifier.fillMaxWidth()); Text("Camera occupata · attendi il completamento") }
        state.preview?.let { Image(it.asImageBitmap(),"Anteprima camera",modifier=Modifier.fillMaxWidth().heightIn(max=300.dp)) }
        state.lastFile?.let { path ->
            Text("Salvato: ${path.substringAfterLast('/')}",style=MaterialTheme.typography.bodySmall)
            OutlinedButton(onClick=onExport) { Text("Esporta ultimo scatto") }
        }
        Text(state.cameraState,style=MaterialTheme.typography.bodySmall,color=MaterialTheme.colorScheme.primary)
        state.logs.lastOrNull()?.let { Text(it,style=MaterialTheme.typography.bodySmall) }
    }
}

@Composable
private fun Selector(label: String,values: List<String>,selected: Int,enabled: Boolean,onSelect: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Column {
        Text(label,style=MaterialTheme.typography.labelLarge)
        Box {
            OutlinedButton(onClick={expanded=true},enabled=enabled,modifier=Modifier.fillMaxWidth()) {
                Text(values.getOrElse(selected) {"Non disponibile"})
            }
            DropdownMenu(expanded,onDismissRequest={expanded=false}) {
                values.forEachIndexed { index,text -> DropdownMenuItem(text={Text(text)},onClick={onSelect(index);expanded=false}) }
            }
        }
    }
}
