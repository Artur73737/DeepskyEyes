package com.deepskyeyes.android.camera

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.ImageFormat
import android.graphics.Rect
import android.hardware.camera2.*
import android.hardware.camera2.params.*
import android.media.Image
import android.media.ImageReader
import android.util.Log
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
    private var rawPreviewReader: ImageReader? = null
    private var sessionStream: JSONObject? = null
    private var selected: JSONObject? = null
    private var outcome: JSONObject? = null
    @Volatile private var liveSettings: String? = null
    private val liveLock = Any()
    private val liveImages = linkedMapOf<Long, Image>()
    private val liveResults = linkedMapOf<Long, TotalCaptureResult>()
    private val liveFrames = java.util.concurrent.ArrayBlockingQueue<Pair<Image, TotalCaptureResult>>(1)
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
            // Configuration validates intent. Capture/preview choose their own
            // surface; do not rebuild a RAW session on every slider edit.
            outcome = accepted
        } catch(e: Exception) { outcome = null; closeSession(); throw e }
        status("READY · ${stream.getInt("width")} × ${stream.getInt("height")}")
        return accepted.copyJson()
    }

    private fun closeSession() {
        liveSettings = null
        session?.close(); session = null
        synchronized(liveLock) {
            liveImages.values.forEach { it.close() }; liveImages.clear(); liveResults.clear()
            liveFrames.poll()?.first?.close()
        }
        reader?.close(); reader = null
        rawPreviewReader?.close(); rawPreviewReader = null
        sessionStream = null
    }
    private fun ensureSession(stream: JSONObject, rawPreview: Boolean = false) {
        if (session != null && sessionStream?.sameJson(stream) == true && (rawPreviewReader != null) == rawPreview) return
        val t0 = SystemClock.elapsedRealtime()
        Log.d("DSKY", "ensureSession ${stream.getInt("width")}x${stream.getInt("height")} rawPreview=$rawPreview")
        closeSession()
        val camera = device ?: fault("Disconnected","Camera is closed")
        val format = when(stream.getString("format")) {
            "Raw16Le", "Dng" -> ImageFormat.RAW_SENSOR
            "Jpeg" -> ImageFormat.JPEG
            "Gray8", "Rgb8" -> ImageFormat.YUV_420_888
            else -> fault("Unsupported","Output format")
        }
        val newReader = ImageReader.newInstance(stream.getInt("width"),stream.getInt("height"),format,4)
        reader = newReader
        newReader.setOnImageAvailableListener({ r ->
            try {
                val image = r.acquireNextImage() ?: return@setOnImageAvailableListener
                if (r !== reader) { image.close(); return@setOnImageAvailableListener }
                if (liveSettings != null) {
                    synchronized(liveLock) { liveImages.put(image.timestamp,image)?.close(); pairLiveFrames() }
                    return@setOnImageAvailableListener
                }
                val target = pendingImage
                if (target == null || !target.complete(image)) image.close()
            } catch(e: Exception) { pendingImage?.completeExceptionally(e) }
        },handler)
        val output = OutputConfiguration(newReader.surface)
        if (stream.getString("pixel_mode") == "MaximumResolution") output.addSensorPixelModeUsed(CameraMetadata.SENSOR_PIXEL_MODE_MAXIMUM_RESOLUTION)
        val outputs = mutableListOf(output)
        if (rawPreview) {
            // Pixel's processed-only stream may report a long exposure while
            // delivering a short frame. Include RAW in the same sensor request
            // so the YUV companion is produced by the still-capture pipeline.
            val raw = discovery.describe(camera.id).getJSONArray("streams").objects().firstOrNull {
                it.getString("format") in listOf("Dng","Raw16Le") && it.getString("pixel_mode") == "Default"
            } ?: fault("Unsupported","Long color preview requires an announced RAW companion stream")
            val rawReader = ImageReader.newInstance(raw.getInt("width"),raw.getInt("height"),ImageFormat.RAW_SENSOR,2)
            rawPreviewReader = rawReader
            rawReader.setOnImageAvailableListener({ r -> runCatching { r.acquireLatestImage()?.close() } },handler)
            outputs.add(OutputConfiguration(rawReader.surface))
        }
        val ready = CompletableFuture<CameraCaptureSession>()
        val config = SessionConfiguration(SessionConfiguration.SESSION_REGULAR,outputs, { runnable -> handler.post(runnable); Unit },
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(s: CameraCaptureSession) { if (!ready.complete(s)) s.close() }
                override fun onConfigureFailed(s: CameraCaptureSession) { s.close(); ready.completeExceptionally(CameraFault("Unsupported","Stream session rejected by Camera2")) }
            })
        try {
            camera.createCaptureSession(config)
            session = await(ready,10_000)
            sessionStream = stream.copyJson()
            Log.d("DSKY", "session ready in ${SystemClock.elapsedRealtime()-t0} ms")
        } catch(e: Exception) { ready.completeExceptionally(e); closeSession(); throw e }
    }

    private fun buildRequest(settings: JSONObject, preview: Boolean): CaptureRequest.Builder {
        val camera = device ?: fault("Disconnected","Camera is closed")
        val b = camera.createCaptureRequest(CameraDevice.TEMPLATE_MANUAL)
        b.set(CaptureRequest.CONTROL_CAPTURE_INTENT, if(preview && settings.getLong("exposure_ns") < 100_000_000L)
            CameraMetadata.CONTROL_CAPTURE_INTENT_PREVIEW else CameraMetadata.CONTROL_CAPTURE_INTENT_STILL_CAPTURE)
        b.addTarget(reader!!.surface)
        rawPreviewReader?.let { b.addTarget(it.surface) }
        b.set(CaptureRequest.CONTROL_MODE, CameraMetadata.CONTROL_MODE_AUTO)
        val stream = sessionStream!!
        b.set(CaptureRequest.SENSOR_PIXEL_MODE, if(stream.getString("pixel_mode") == "MaximumResolution") CameraMetadata.SENSOR_PIXEL_MODE_MAXIMUM_RESOLUTION else CameraMetadata.SENSOR_PIXEL_MODE_DEFAULT)
        run {
            b.set(CaptureRequest.CONTROL_AE_MODE, CameraMetadata.CONTROL_AE_MODE_OFF)
            b.set(CaptureRequest.SENSOR_EXPOSURE_TIME,settings.getLong("exposure_ns"))
            b.set(CaptureRequest.SENSOR_SENSITIVITY,settings.getInt("sensitivity"))
            b.set(CaptureRequest.SENSOR_FRAME_DURATION,maxOf(settings.getLong("exposure_ns"),
                settings.longOrNull("frame_duration_ns") ?: 0L,
                stream.longOrNull("min_frame_duration_ns") ?: 0L))
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

    private fun take(settings: JSONObject, preview: Boolean, warmupLeft: Int = 5): Pair<Image,TotalCaptureResult> {
        requireCamera(!stopped, "Disconnected", "Camera stopped")
        if(power.currentThermalStatus >= PowerManager.THERMAL_STATUS_SEVERE) fault("Thermal","Android thermal severity refuses capture")
        val imageFuture = CompletableFuture<Image>(); val resultFuture = CompletableFuture<TotalCaptureResult>()
        pendingImage = imageFuture; pendingResult = resultFuture
        // First valid RAW after HAL startup can span a pipeline frame plus
        // the requested exposure. This is a timeout, never an inserted delay.
        val timeout = ((maxOf(settings.longOrNull("frame_duration_ns") ?: 0L,settings.getLong("exposure_ns"))) / 1_000_000 * 2 + 15_000).coerceAtMost(115_000)
        var image: Image? = null
        try {
            val request = buildRequest(settings,preview).build()
            val submitted = SystemClock.elapsedRealtimeNanos()
            Log.d("DSKY", "take submit exp=${settings.getLong("exposure_ns")} warmupLeft=$warmupLeft")
            session!!.capture(request, object : CameraCaptureSession.CaptureCallback() {
                override fun onCaptureCompleted(s: CameraCaptureSession,request: CaptureRequest,result: TotalCaptureResult) { resultFuture.complete(result) }
                override fun onCaptureFailed(s: CameraCaptureSession,request: CaptureRequest,failure: CaptureFailure) { resultFuture.completeExceptionally(CameraFault("Io","Capture failure ${failure.reason}")) }
                override fun onCaptureSequenceAborted(s: CameraCaptureSession,id: Int) { resultFuture.completeExceptionally(CameraFault("Io","Capture aborted")) }
            },handler)
            val deadline = SystemClock.elapsedRealtime() + timeout
            val result = await(resultFuture,timeout)
            Log.d("DSKY", "take result after ${(SystemClock.elapsedRealtimeNanos()-submitted)/1_000_000} ms (exp=${settings.getLong("exposure_ns")})")
            image = await(imageFuture,(deadline-SystemClock.elapsedRealtime()).coerceAtLeast(1))
            Log.d("DSKY", "take image after ${(SystemClock.elapsedRealtimeNanos()-submitted)/1_000_000} ms")
            val timestamp = result[CaptureResult.SENSOR_TIMESTAMP] ?: fault("Io","Sensor timestamp missing")
            requireCamera(image.timestamp == timestamp, "Io", "Image and CaptureResult timestamps differ")
            val reportedExposure = result[CaptureResult.SENSOR_EXPOSURE_TIME] ?: fault("Io","Sensor exposure missing")
            // Some Pixel HALs return startup frames carrying the new
            // long-exposure metadata before that exposure could physically
            // finish. Never save or display such a frame as valid: discard
            // and re-acquire on the (now primed) session, bounded retries.
            if (reportedExposure >= 100_000_000L && SystemClock.elapsedRealtimeNanos()-submitted < reportedExposure*95/100) {
                image.close(); image = null
                requireCamera(warmupLeft > 0,"Io","Camera returned a frame before its reported exposure could finish")
                Log.d("DSKY", "take warmup-discard elapsedMs=${(SystemClock.elapsedRealtimeNanos()-submitted)/1_000_000} warmupLeft=$warmupLeft")
                status("Discarding unverified startup frame; acquiring real exposure")
                pendingImage = null; pendingResult = null
                return take(settings,preview,warmupLeft-1)
            }
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
            requireCamera(image.width == stream.getInt("width") && image.height == stream.getInt("height"),
                "Io","Camera returned dimensions different from the selected full stream; refusing cropped or resized data")
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

    private fun pairLiveFrames() {
        for (timestamp in liveImages.keys.toList()) {
            val result = liveResults.remove(timestamp) ?: continue
            val image = liveImages.remove(timestamp)!!
            liveFrames.poll()?.first?.close()
            liveFrames.offer(image to result)
        }
        while(liveImages.size > 1) liveImages.remove(liveImages.keys.first())?.close()
        while(liveResults.size > 4) liveResults.remove(liveResults.keys.first())
    }

    @Synchronized fun previewFrame(): Capture {
        val configured = outcome ?: fault("InvalidState","Configure before preview")
        val settings = configured.getJSONObject("applied").getJSONObject("settings")
        val previews = discovery.describe(selected!!.getString("camera_id")).getJSONArray("preview_streams").objects()
        val stream = previews.filter { it.getInt("width") <= 640 }.maxByOrNull { it.getInt("width") * it.getInt("height") } ?: previews.firstOrNull() ?: fault("Unsupported","No bounded YUV preview stream")
        val longExposure = settings.getLong("exposure_ns") >= 100_000_000L
        ensureSession(stream,longExposure)
        val key = settings.toString()
        if (!longExposure && liveSettings != key) {
            // Recreate only when controls change, flushing frames from the old
            // request so a 16-second frame cannot masquerade as a new short one.
            if (liveSettings != null) { closeSession(); ensureSession(stream) }
            liveSettings = key
            session!!.setRepeatingRequest(buildRequest(settings,true).build(), object : CameraCaptureSession.CaptureCallback() {
                override fun onCaptureCompleted(s: CameraCaptureSession,request: CaptureRequest,result: TotalCaptureResult) {
                    if (s !== session) return
                    synchronized(liveLock) { result[CaptureResult.SENSOR_TIMESTAMP]?.let { liveResults[it] = result }; pairLiveFrames() }
                }
            },handler)
        }
        val timeout = (settings.getLong("exposure_ns") / 1_000_000 * 2 + 15_000).coerceAtMost(115_000)
        val (image,result) = if (longExposure) {
            if (liveSettings != null) { closeSession(); ensureSession(stream,true) }
            take(settings,false)
        } else liveFrames.poll(timeout,TimeUnit.MILLISECONDS) ?: fault("Timeout","Live preview callback timeout")
        image.use {
            val planes = image.planes
            val width = image.width; val height = image.height
            // Densify each plane with one native memcpy per row instead of
            // one JNI ByteBuffer.get() call per pixel (~1.2M calls/frame at
            // 640x480 was the liveview bottleneck at short exposures).
            // Row-bounded reads: remaining() can be smaller than
            // rowStride*height, so never bulk-read past the row limit.
            fun readPlane(p: Int, w: Int, h: Int): ByteArray {
                val buf = planes[p].buffer.duplicate()
                val rowStride = planes[p].rowStride
                val pixStride = planes[p].pixelStride
                val out = ByteArray(w * h)
                val row = ByteArray(rowStride)
                for (y in 0 until h) {
                    buf.position(y * rowStride)
                    val n = minOf(rowStride, buf.remaining())
                    requireCamera(n >= (w-1)*pixStride+1,"Io","Truncated YUV plane row; refusing fabricated pixels")
                    buf.get(row, 0, n)
                    var x = 0; var i = 0
                    while (x < w && i < n) { out[y * w + x] = row[i]; x++; i += pixStride }
                }
                return out
            }
            val yPlane = readPlane(0, width, height)
            val uPlane = readPlane(1, width / 2, height / 2)
            val vPlane = readPlane(2, width / 2, height / 2)
            val bytes = ByteArray(width*height*3)
            for (y in 0 until height) {
                val yOff = y * width
                val uvOff = (y / 2) * (width / 2)
                var o = y * width * 3
                for (x in 0 until width) {
                    val l = (yPlane[yOff + x].toInt() and 255) - 16
                    val yy = if (l < 0) 0 else l
                    val xx = uvOff + x / 2
                    val u = (uPlane[xx].toInt() and 255) - 128
                    val v = (vPlane[xx].toInt() and 255) - 128
                    bytes[o++] = ((298 * yy + 409 * v + 128) shr 8).coerceIn(0, 255).toByte()
                    bytes[o++] = ((298 * yy - 100 * u - 208 * v + 128) shr 8).coerceIn(0, 255).toByte()
                    bytes[o++] = ((298 * yy + 516 * u + 128) shr 8).coerceIn(0, 255).toByte()
                }
            }
            return Capture(obj("width" to image.width,"height" to image.height,
                "format" to "Rgb8","timestamp_ns" to result[CaptureResult.SENSOR_TIMESTAMP],"origin" to "Device",
                "reported_exposure_ns" to result[CaptureResult.SENSOR_EXPOSURE_TIME],
                "reported_sensitivity" to result[CaptureResult.SENSOR_SENSITIVITY]),bytes)
        }
    }

    @Synchronized fun preview(): JSONObject = previewFrame().let {
        it.metadata.put("payload",arr(it.bytes.map { b -> b.toInt() and 255 }))
    }

    /** One-shot center AF; return only a measured, successfully locked distance. */
    @Synchronized fun autofocusCenter(): Long {
        val settings = outcome?.getJSONObject("applied")?.getJSONObject("settings")
            ?: fault("InvalidState","Configure before autofocus")
        val c = discovery.characteristics(selected!!.getString("camera_id"))
        requireCamera(c[CameraCharacteristics.CONTROL_AF_AVAILABLE_MODES]?.contains(CameraMetadata.CONTROL_AF_MODE_AUTO) == true,
            "Unsupported","One-shot autofocus is not supported")
        requireCamera((c[CameraCharacteristics.CONTROL_MAX_REGIONS_AF] ?: 0) > 0,"Unsupported","Central AF region is not supported")
        val sensor = c[CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE] ?: fault("Unsupported","AF coordinate bounds unavailable")
        val previews = discovery.describe(selected!!.getString("camera_id")).getJSONArray("preview_streams").objects()
        val stream = previews.filter { it.getInt("width") <= 640 }.maxByOrNull { it.getInt("width")*it.getInt("height") }
            ?: fault("Unsupported","AF preview surface unavailable")
        closeSession(); ensureSession(stream)
        val result = CompletableFuture<Long>()
        val b = buildRequest(settings,true)
        b.set(CaptureRequest.CONTROL_AE_MODE,CameraMetadata.CONTROL_AE_MODE_ON)
        b.set(CaptureRequest.CONTROL_AF_MODE,CameraMetadata.CONTROL_AF_MODE_AUTO)
        val half = (minOf(sensor.width(),sensor.height()) / 10).coerceAtLeast(1)
        b.set(CaptureRequest.CONTROL_AF_REGIONS,arrayOf(MeteringRectangle(Rect(sensor.centerX()-half,sensor.centerY()-half,
            sensor.centerX()+half,sensor.centerY()+half),MeteringRectangle.METERING_WEIGHT_MAX)))
        val callback = object : CameraCaptureSession.CaptureCallback() {
            override fun onCaptureCompleted(s: CameraCaptureSession,request: CaptureRequest,r: TotalCaptureResult) {
                when(r[CaptureResult.CONTROL_AF_STATE]) {
                    CameraMetadata.CONTROL_AF_STATE_FOCUSED_LOCKED -> r[CaptureResult.LENS_FOCUS_DISTANCE]?.let { result.complete((it*1000).toLong()) }
                    CameraMetadata.CONTROL_AF_STATE_NOT_FOCUSED_LOCKED -> result.completeExceptionally(CameraFault("InvalidState","Center AF could not find focus; increase illumination or focus manually"))
                }
            }
            override fun onCaptureFailed(s: CameraCaptureSession,request: CaptureRequest,failure: CaptureFailure) {
                result.completeExceptionally(CameraFault("Io","Autofocus capture failed"))
            }
        }
        try {
            b.set(CaptureRequest.CONTROL_AF_TRIGGER,CameraMetadata.CONTROL_AF_TRIGGER_IDLE)
            session!!.setRepeatingRequest(b.build(),callback,handler)
            b.set(CaptureRequest.CONTROL_AF_TRIGGER,CameraMetadata.CONTROL_AF_TRIGGER_START)
            session!!.capture(b.build(),callback,handler)
            val distance = await(result,10_000)
            // Freeze the measured result immediately in the engine, not only
            // after a later desktop configure which might fail or be delayed.
            settings.put("focus",obj("Manual" to obj("millidiopters" to distance,"locked" to true)))
            return distance
        } finally { closeSession() }
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
        val p = image.planes[0]
        return RawPacking.pack(p.buffer,image.width,image.height,p.rowStride,p.pixelStride)
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
