package com.deepskyeyes.android.camera

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.ImageFormat
import android.graphics.Rect
import android.hardware.camera2.*
import android.hardware.camera2.params.*
import android.media.Image
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.os.PowerManager
import android.os.SystemClock
import android.util.Rational
import com.deepskyeyes.android.protocol.*
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import java.util.concurrent.atomic.AtomicLong

/** Called only on the RPC worker. Camera callbacks run on a separate HandlerThread. */
class CameraEngine(context: Context, private val status: (String) -> Unit) : AutoCloseable {
    private val manager = context.getSystemService(CameraManager::class.java)
    private val power = context.getSystemService(PowerManager::class.java)
    val discovery = CapabilityDiscovery(manager)
    private val thread = HandlerThread("Camera2-callbacks").apply { start() }
    private val handler = Handler(thread.looper)
    @Volatile private var device: CameraDevice? = null
    @Volatile private var session: CameraCaptureSession? = null
    private var reader: ImageReader? = null
    private var sessionStream: JSONObject? = null
    private var selected: JSONObject? = null
    private var outcome: JSONObject? = null
    private val frameCounter = AtomicLong()
    @Volatile private var pendingResult: CompletableFuture<TotalCaptureResult>? = null
    @Volatile private var pendingImage: CompletableFuture<Image>? = null
    @Volatile private var stopped = false
    val identity get() = discovery.identity
    data class Capture(val metadata: JSONObject, val bytes: ByteArray)

    @SuppressLint("MissingPermission")
    @Synchronized
    fun open(selection: JSONObject) {
        requireCamera(device == null, "InvalidState", "Camera already open")
        requireCamera(selection.isNull("physical_id"), "Unsupported", "Physical routing is not advertised")
        val id = selection.getString("camera_id")
        requireCamera(id in manager.cameraIdList, "Unsupported", "Camera ID not enumerated")
        val future = CompletableFuture<CameraDevice>()
        stopped = false
        manager.openCamera(id, object : CameraDevice.StateCallback() {
            override fun onOpened(camera: CameraDevice) {
                if (future.isDone || stopped) camera.close() else { device = camera; future.complete(camera) }
            }
            override fun onDisconnected(camera: CameraDevice) { camera.close(); device = null; failPending("Disconnected", "Camera disconnected"); future.completeExceptionally(CameraFault("Disconnected","Camera disconnected")); status("DISCONNECTED") }
            override fun onError(camera: CameraDevice, error: Int) { camera.close(); device = null; failPending("Io","Camera2 error $error"); future.completeExceptionally(CameraFault("Io","Camera2 error $error")); status("CAMERA ERROR $error") }
        }, handler)
        try { await(future, 10_000) } catch(e: Exception) { future.completeExceptionally(e); device?.close(); device = null; throw e }
        selected = selection.copyJson()
        status("OPEN · $id")
    }

    @Synchronized fun configure(request: JSONObject, policy: String): JSONObject {
        val selection = selected ?: fault("InvalidState","Open a camera first")
        requireCamera(request.getJSONObject("selection").sameJson(selection), "InvalidRequest", "Configuration selects a different camera")
        val caps = discovery.describe(selection.getString("camera_id"))
        val accepted = RequestValidator.validate(caps, request, policy)
        val settings = accepted.getJSONObject("applied").getJSONObject("settings")
        val stream = settings.getJSONObject("stream")
        val size = stream.getLong("width") * stream.getLong("height") * 2
        requireCamera(size <= minOf(128L * 1024 * 1024, Runtime.getRuntime().maxMemory() / 3), "OutOfRange", "Stream exceeds safe RAW memory budget")
        try {
            ensureSession(stream)
            buildRequest(settings, false).build()
            outcome = accepted
        } catch(e: Exception) { outcome = null; closeSession(); throw e }
        status("READY · ${stream.getInt("width")} × ${stream.getInt("height")}")
        return accepted.copyJson()
    }

    private fun closeSession() {
        session?.close(); session = null
        reader?.close(); reader = null
        sessionStream = null
    }
    private fun ensureSession(stream: JSONObject) {
        if (session != null && sessionStream?.sameJson(stream) == true) return
        closeSession()
        val camera = device ?: fault("Disconnected","Camera is closed")
        val format = when(stream.getString("format")) {
            "Raw16Le", "Dng" -> ImageFormat.RAW_SENSOR
            "Jpeg" -> ImageFormat.JPEG
            "Gray8" -> ImageFormat.YUV_420_888
            else -> fault("Unsupported","Output format")
        }
        val newReader = ImageReader.newInstance(stream.getInt("width"),stream.getInt("height"),format,2)
        reader = newReader
        newReader.setOnImageAvailableListener({ r ->
            try {
                val image = r.acquireNextImage() ?: return@setOnImageAvailableListener
                val target = pendingImage
                if (target == null || !target.complete(image)) image.close()
            } catch(e: Exception) { pendingImage?.completeExceptionally(e) }
        },handler)
        val output = OutputConfiguration(newReader.surface)
        if (stream.getString("pixel_mode") == "MaximumResolution") output.addSensorPixelModeUsed(CameraMetadata.SENSOR_PIXEL_MODE_MAXIMUM_RESOLUTION)
        val ready = CompletableFuture<CameraCaptureSession>()
        val config = SessionConfiguration(SessionConfiguration.SESSION_REGULAR,listOf(output), { runnable -> handler.post(runnable); Unit },
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(s: CameraCaptureSession) { if (!ready.complete(s)) s.close() }
                override fun onConfigureFailed(s: CameraCaptureSession) { s.close(); ready.completeExceptionally(CameraFault("Unsupported","Stream session rejected by Camera2")) }
            })
        try {
            camera.createCaptureSession(config)
            session = await(ready,10_000)
            sessionStream = stream.copyJson()
        } catch(e: Exception) { ready.completeExceptionally(e); closeSession(); throw e }
    }

    private fun buildRequest(settings: JSONObject, preview: Boolean): CaptureRequest.Builder {
        val camera = device ?: fault("Disconnected","Camera is closed")
        val b = camera.createCaptureRequest(if(preview) CameraDevice.TEMPLATE_PREVIEW else CameraDevice.TEMPLATE_STILL_CAPTURE)
        b.addTarget(reader!!.surface)
        b.set(CaptureRequest.CONTROL_MODE, CameraMetadata.CONTROL_MODE_AUTO)
        val stream = sessionStream!!
        b.set(CaptureRequest.SENSOR_PIXEL_MODE, if(stream.getString("pixel_mode") == "MaximumResolution") CameraMetadata.SENSOR_PIXEL_MODE_MAXIMUM_RESOLUTION else CameraMetadata.SENSOR_PIXEL_MODE_DEFAULT)
        if (preview) {
            b.set(CaptureRequest.CONTROL_AE_MODE, CameraMetadata.CONTROL_AE_MODE_ON)
        } else {
            b.set(CaptureRequest.CONTROL_AE_MODE, CameraMetadata.CONTROL_AE_MODE_OFF)
            b.set(CaptureRequest.SENSOR_EXPOSURE_TIME,settings.getLong("exposure_ns"))
            b.set(CaptureRequest.SENSOR_SENSITIVITY,settings.getInt("sensitivity"))
            settings.longOrNull("frame_duration_ns")?.let { b.set(CaptureRequest.SENSOR_FRAME_DURATION,it) }
        }
        settings.optJSONObject("focus")?.getJSONObject("Manual")?.let {
            b.set(CaptureRequest.CONTROL_AF_MODE,CameraMetadata.CONTROL_AF_MODE_OFF)
            b.set(CaptureRequest.LENS_FOCUS_DISTANCE,it.getLong("millidiopters") / 1000f)
        }
        settings.longOrNull("zoom_x1000")?.let { b.set(CaptureRequest.CONTROL_ZOOM_RATIO,it / 1000f) }
        settings.optJSONObject("crop")?.let { b.set(CaptureRequest.SCALER_CROP_REGION,Rect(it.getInt("x"),it.getInt("y"),it.getInt("x")+it.getInt("width"),it.getInt("y")+it.getInt("height"))) }
        settings.optJSONObject("white_balance")?.let { wb ->
            if (wb.has("Mode")) {
                val mode = CapabilityDiscovery.AWB.entries.firstOrNull { it.value == wb.getString("Mode") }?.key ?: fault("Unsupported","WB mode")
                b.set(CaptureRequest.CONTROL_AWB_MODE,mode)
            } else if (wb.has("Manual")) {
                val manual = wb.getJSONObject("Manual"); val gains = manual.getJSONArray("gains_x1000")
                val matrix = manual.getJSONArray("transform_millionths")
                b.set(CaptureRequest.CONTROL_AWB_MODE,CameraMetadata.CONTROL_AWB_MODE_OFF)
                b.set(CaptureRequest.COLOR_CORRECTION_MODE,CameraMetadata.COLOR_CORRECTION_MODE_TRANSFORM_MATRIX)
                b.set(CaptureRequest.COLOR_CORRECTION_GAINS,RggbChannelVector(gains.getLong(0)/1000f,gains.getLong(1)/1000f,gains.getLong(2)/1000f,gains.getLong(3)/1000f))
                b.set(CaptureRequest.COLOR_CORRECTION_TRANSFORM,ColorSpaceTransform(Array(9) { Rational(matrix.getInt(it),1_000_000) }))
            }
        }
        val modes = settings.optJSONObject("processing") ?: JSONObject()
        for ((name,key) in PROCESSING_KEYS) if(modes.has(name)) {
            val table = if(name == "tonemap") CapabilityDiscovery.TONEMAP else CapabilityDiscovery.PROCESSING
            b.set(key,table.entries.first { it.value == modes.getString(name) }.key)
        }
        settings.stringOrNull("ois")?.let { b.set(CaptureRequest.LENS_OPTICAL_STABILIZATION_MODE,if(it == "off") 0 else 1) }
        settings.stringOrNull("eis")?.let { b.set(CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE,if(it == "off") 0 else 1) }
        val c = discovery.characteristics(camera.id)
        if(c[CameraCharacteristics.STATISTICS_INFO_AVAILABLE_LENS_SHADING_MAP_MODES]?.contains(CameraMetadata.STATISTICS_LENS_SHADING_MAP_MODE_ON) == true) {
            b.set(CaptureRequest.STATISTICS_LENS_SHADING_MAP_MODE,CameraMetadata.STATISTICS_LENS_SHADING_MAP_MODE_ON)
        }
        return b
    }

    private fun take(settings: JSONObject, preview: Boolean): Pair<Image,TotalCaptureResult> {
        requireCamera(!stopped, "Disconnected", "Camera stopped")
        if(power.currentThermalStatus >= PowerManager.THERMAL_STATUS_SEVERE) fault("Thermal","Android thermal severity refuses capture")
        val imageFuture = CompletableFuture<Image>(); val resultFuture = CompletableFuture<TotalCaptureResult>()
        pendingImage = imageFuture; pendingResult = resultFuture
        val timeout = if(preview) 10_000L else ((settings.longOrNull("frame_duration_ns") ?: settings.getLong("exposure_ns")) / 1_000_000 + 15_000).coerceAtMost(115_000)
        var image: Image? = null
        try {
            val request = buildRequest(settings,preview).build()
            session!!.capture(request, object : CameraCaptureSession.CaptureCallback() {
                override fun onCaptureCompleted(s: CameraCaptureSession,request: CaptureRequest,result: TotalCaptureResult) { resultFuture.complete(result) }
                override fun onCaptureFailed(s: CameraCaptureSession,request: CaptureRequest,failure: CaptureFailure) { resultFuture.completeExceptionally(CameraFault("Io","Capture failure ${failure.reason}")) }
                override fun onCaptureSequenceAborted(s: CameraCaptureSession,id: Int) { resultFuture.completeExceptionally(CameraFault("Io","Capture aborted")) }
            },handler)
            val deadline = SystemClock.elapsedRealtime() + timeout
            val result = await(resultFuture,timeout)
            image = await(imageFuture,(deadline-SystemClock.elapsedRealtime()).coerceAtLeast(1))
            val timestamp = result[CaptureResult.SENSOR_TIMESTAMP] ?: fault("Io","Sensor timestamp missing")
            requireCamera(image.timestamp == timestamp, "Io", "Image and CaptureResult timestamps differ")
            return image to result
        } catch(e: Exception) {
            image?.close()
            if(image == null && !imageFuture.isCompletedExceptionally) imageFuture.getNow(null)?.close()
            imageFuture.completeExceptionally(e); resultFuture.completeExceptionally(e)
            closeSession()
            throw e
        } finally { pendingImage = null; pendingResult = null }
    }

    @Synchronized fun capture(): Capture {
        val configured = outcome?.copyJson() ?: fault("InvalidState","Configure before capture")
        val settings = configured.getJSONObject("applied").getJSONObject("settings")
        val stream = settings.getJSONObject("stream")
        ensureSession(stream)
        status("CAPTURING")
        val (image,result) = take(settings,false)
        image.use {
            val format = stream.getString("format")
            val c = discovery.characteristics(selected!!.getString("camera_id"))
            val data = when(format) {
                "Dng" -> ByteArrayOutputStream().use { output -> DngCreator(c,result).use { it.writeImage(output,image) }; output.toByteArray() }
                "Raw16Le" -> packedRaw(image)
                "Jpeg" -> ByteArray(image.planes[0].buffer.remaining()).also { image.planes[0].buffer.get(it) }
                else -> fault("Unsupported","Capture output")
            }
            val reported = reported(result,stream)
            val timestamp = result[CaptureResult.SENSOR_TIMESTAMP]!!
            val exposure = result[CaptureResult.SENSOR_EXPOSURE_TIME] ?: fault("Io","CaptureResult exposure missing")
            val white = result[CaptureResult.SENSOR_DYNAMIC_WHITE_LEVEL] ?: c[CameraCharacteristics.SENSOR_INFO_WHITE_LEVEL]
            val black = result[CaptureResult.SENSOR_DYNAMIC_BLACK_LEVEL]?.map { it.toInt() }
                ?: c[CameraCharacteristics.SENSOR_BLACK_LEVEL_PATTERN]?.let { p -> listOf(p.getOffsetForIndex(0,0),p.getOffsetForIndex(1,0),p.getOffsetForIndex(0,1),p.getOffsetForIndex(1,1)) }
            val cfa = when(c[CameraCharacteristics.SENSOR_INFO_COLOR_FILTER_ARRANGEMENT]) { 0 -> "RGGB"; 1 -> "GRBG"; 2 -> "GBRG"; 3 -> "BGGR"; else -> null }
            val layout = if(format == "Raw16Le") obj("row_stride_bytes" to image.width*2,"pixel_stride_bytes" to 2,
                "bit_depth" to 16,"cfa" to cfa,"black_levels" to black?.let(::arr),"white_level" to white) else null
            val metadata = obj("identity" to identity,"origin" to "Device","frame_id" to frameCounter.incrementAndGet(),
                "configuration" to configured,"reported" to reported,"active_physical_id" to result[CaptureResult.LOGICAL_MULTI_CAMERA_ACTIVE_PHYSICAL_ID],
                "timestamp_domain" to if(c[CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE] == CameraMetadata.SENSOR_INFO_TIMESTAMP_SOURCE_REALTIME) "Android elapsedRealtimeNanos" else "Android sensor UNKNOWN clock",
                "start_ns" to timestamp,"end_ns" to Math.addExact(timestamp,exposure),"raw_layout" to layout)
            status("READY · frame ${metadata.getLong("frame_id")} · ${data.size} bytes")
            return Capture(metadata,data)
        }
    }

    @Synchronized fun preview(): JSONObject {
        val configured = outcome ?: fault("InvalidState","Configure before preview")
        val settings = configured.getJSONObject("applied").getJSONObject("settings")
        val previews = discovery.describe(selected!!.getString("camera_id")).getJSONArray("preview_streams").objects()
        val stream = previews.filter { it.getInt("width") <= 640 }.lastOrNull() ?: previews.firstOrNull() ?: fault("Unsupported","No bounded YUV preview stream")
        ensureSession(stream)
        val (image,result) = take(settings,true)
        image.use {
            val plane = image.planes[0]; val buffer = plane.buffer
            val bytes = ByteArray(image.width*image.height)
            for(y in 0 until image.height) for(x in 0 until image.width) bytes[y*image.width+x] = buffer.get(y*plane.rowStride+x*plane.pixelStride)
            return obj("payload" to arr(bytes.map { it.toInt() and 255 }),"width" to image.width,"height" to image.height,
                "format" to "Gray8","timestamp_ns" to result[CaptureResult.SENSOR_TIMESTAMP],"origin" to "Device")
        }
    }

    private fun reported(r: TotalCaptureResult, stream: JSONObject): JSONObject {
        val focus = r[CaptureResult.LENS_FOCUS_DISTANCE]
        val af = r[CaptureResult.CONTROL_AF_MODE]
        val lensState = r[CaptureResult.LENS_STATE]
        val wb = r[CaptureResult.CONTROL_AWB_MODE]?.let { mode -> CapabilityDiscovery.AWB[mode]?.let { obj("Mode" to it) } }
        val crop = r[CaptureResult.SCALER_CROP_REGION]
        val processing = JSONObject()
        val resultKeys = mapOf("edge" to CaptureResult.EDGE_MODE,"noise_reduction" to CaptureResult.NOISE_REDUCTION_MODE,
            "hot_pixel" to CaptureResult.HOT_PIXEL_MODE,"shading" to CaptureResult.SHADING_MODE,"tonemap" to CaptureResult.TONEMAP_MODE,
            "aberration" to CaptureResult.COLOR_CORRECTION_ABERRATION_MODE,"distortion" to CaptureResult.DISTORTION_CORRECTION_MODE)
        for((name,key) in resultKeys) r[key]?.let { value ->
            val table = if(name == "tonemap") CapabilityDiscovery.TONEMAP else CapabilityDiscovery.PROCESSING
            table[value]?.let { processing.put(name,it) }
        }
        return obj("exposure_ns" to r[CaptureResult.SENSOR_EXPOSURE_TIME],"sensitivity" to r[CaptureResult.SENSOR_SENSITIVITY],
            "frame_duration_ns" to r[CaptureResult.SENSOR_FRAME_DURATION],
            "focus" to if(focus != null && af == CameraMetadata.CONTROL_AF_MODE_OFF) obj("Manual" to obj("millidiopters" to (focus*1000).toLong(),"locked" to (lensState == CameraMetadata.LENS_STATE_STATIONARY))) else null,
            "zoom_x1000" to r[CaptureResult.CONTROL_ZOOM_RATIO]?.let { (it*1000).toLong() },
            "crop" to crop?.let { obj("x" to it.left,"y" to it.top,"width" to it.width(),"height" to it.height()) },
            "stream" to stream.copyJson(),"white_balance" to wb,"processing" to processing,
            "ois" to r[CaptureResult.LENS_OPTICAL_STABILIZATION_MODE]?.let { if(it == 0) "off" else "on" },
            "eis" to r[CaptureResult.CONTROL_VIDEO_STABILIZATION_MODE]?.let { if(it == 0) "off" else "on" })
    }
    private fun packedRaw(image: Image): ByteArray {
        val p = image.planes[0]; val b = p.buffer
        requireCamera(p.pixelStride >= 2,"Unsupported","Unsupported RAW pixel stride")
        val output = ByteArray(image.width*image.height*2)
        for(y in 0 until image.height) for(x in 0 until image.width) {
            val src=y*p.rowStride+x*p.pixelStride; val dst=(y*image.width+x)*2
            output[dst]=b.get(src); output[dst+1]=b.get(src+1)
        }
        return output
    }
    private fun failPending(code: String,message: String) {
        val error = CameraFault(code,message)
        pendingResult?.completeExceptionally(error); pendingImage?.completeExceptionally(error)
    }
    /** May be called from service destruction to unblock capture promptly. */
    fun abort() { stopped = true; failPending("Disconnected","Service stopped"); device?.close(); device = null }
    @Synchronized fun closeCamera() { stopped = true; failPending("Disconnected","Camera closed"); closeSession(); device?.close(); device=null; outcome=null; selected=null; status("CLOSED") }
    override fun close() { closeCamera(); thread.quitSafely() }
    private fun <T> await(future: CompletableFuture<T>,timeoutMs: Long): T = try { future.get(timeoutMs,TimeUnit.MILLISECONDS) }
        catch(e: TimeoutException) { fault("Timeout","Camera2 callback timeout") }
        catch(e: java.util.concurrent.ExecutionException) { throw (e.cause ?: e) }
    companion object {
        private val PROCESSING_KEYS = mapOf("edge" to CaptureRequest.EDGE_MODE,"noise_reduction" to CaptureRequest.NOISE_REDUCTION_MODE,
            "hot_pixel" to CaptureRequest.HOT_PIXEL_MODE,"shading" to CaptureRequest.SHADING_MODE,"tonemap" to CaptureRequest.TONEMAP_MODE,
            "aberration" to CaptureRequest.COLOR_CORRECTION_ABERRATION_MODE,"distortion" to CaptureRequest.DISTORTION_CORRECTION_MODE)
    }
}
