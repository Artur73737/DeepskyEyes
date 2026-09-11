package com.deepskyeyes.android.service

import android.content.Context
import android.graphics.Bitmap
import com.deepskyeyes.android.camera.CameraEngine
import com.deepskyeyes.android.protocol.*
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

/** Local diagnostic capture, using the same camera adapter as the desktop endpoint. */
object LocalCamera {
    fun execute(context: Context,camera: CameraEngine,operation: String,requestText: String?) {
        if(operation == "close") { camera.closeCamera(); return }
        val request = JSONObject(requestText ?: fault("InvalidRequest","Missing capture configuration"))
        camera.closeCamera()
        camera.open(request.getJSONObject("selection"))
        val configured = camera.configure(request,"Reject")
        if(operation == "capture") {
            val directory = File(context.getExternalFilesDir(null),"captures").apply { mkdirs() }
            val stream = configured.getJSONObject("applied").getJSONObject("settings").getJSONObject("stream")
            val estimated = stream.getLong("width") * stream.getLong("height") * 6
            requireCamera(directory.usableSpace > estimated + 32L * 1024 * 1024,"Io","Insufficient free storage")
            val frame = camera.capture()
            val extension = when(stream.getString("format")) { "Dng" -> "dng"; "Raw16Le" -> "raw16"; "Jpeg" -> "jpg"; else -> fault("Unsupported","Capture format") }
            val file = File(directory,"Deepsky_${System.currentTimeMillis()}_${frame.metadata.getLong("frame_id")}.$extension")
            requireCamera(file.createNewFile(),"Io","Capture file already exists")
            file.outputStream().use { it.write(frame.bytes); it.fd.sync() }
            val hash = MessageDigest.getInstance("SHA-256").digest(frame.bytes).joinToString("") { "%02x".format(it.toInt() and 255) }
            val metadata = obj("schema_version" to 1,"sha256" to hash,"size_bytes" to frame.bytes.size,"capture" to frame.metadata)
            File(directory,file.name+".json").writeText(metadata.toString(2))
            BridgeStatus.update { it.copy(lastFile = file.absolutePath,captured = it.captured+1) }
            BridgeStatus.log("Saved ${file.name} · ${frame.bytes.size} bytes · SHA-256 $hash")
        }
        if(operation !in listOf("capture","preview")) fault("Unsupported","Unknown local operation")
        try {
            val preview = camera.preview()
            val w=preview.getInt("width"); val h=preview.getInt("height"); val data=preview.getJSONArray("payload")
            val colors=IntArray(w*h) { 0xff000000.toInt() or (data.getInt(it*3) shl 16) or (data.getInt(it*3+1) shl 8) or data.getInt(it*3+2) }
            val bitmap=Bitmap.createBitmap(colors,w,h,Bitmap.Config.ARGB_8888)
            BridgeStatus.update { it.copy(preview = bitmap) }
        } catch(e: Exception) {
            if(operation == "preview") throw e
            BridgeStatus.log("Capture saved; preview unavailable: ${e.message}")
        }
    }
}
