package com.deepskyeyes.android.camera

import android.graphics.ImageFormat
import android.hardware.camera2.CameraCharacteristics as C
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CameraMetadata
import android.hardware.camera2.params.StreamConfigurationMap
import android.os.Build
import com.deepskyeyes.android.protocol.*
import org.json.JSONArray
import org.json.JSONObject

private fun IntArray?.orEmpty(): IntArray = this ?: intArrayOf()
private inline fun <R: Any> IntArray.mapNotNull(transform: (Int) -> R?): List<R> = asIterable().mapNotNull(transform)

class CapabilityDiscovery(private val manager: CameraManager) {
    val identity: String get() = "${Build.MANUFACTURER} ${Build.MODEL} / Android ${Build.VERSION.RELEASE}"
    fun discover(): JSONArray = arr(manager.cameraIdList.map { describe(it) })
    fun characteristics(id: String): C = manager.getCameraCharacteristics(id)
    fun describe(id: String): JSONObject {
        val c = characteristics(id)
        val capabilities = c[C.REQUEST_AVAILABLE_CAPABILITIES]?.toSet().orEmpty()
        val manual = CameraMetadata.REQUEST_AVAILABLE_CAPABILITIES_MANUAL_SENSOR in capabilities
        val raw = CameraMetadata.REQUEST_AVAILABLE_CAPABILITIES_RAW in capabilities
        val af = c[C.CONTROL_AF_AVAILABLE_MODES].orEmpty()
        val focus = c[C.LENS_INFO_MINIMUM_FOCUS_DISTANCE]
        val streams = mutableListOf<JSONObject>()
        fun collect(map: StreamConfigurationMap?, mode: String) {
            if (map == null) return
            if (raw) for (size in map.getOutputSizes(ImageFormat.RAW_SENSOR).orEmpty()) {
                val pre = if (mode == "MaximumResolution") c[C.SENSOR_INFO_PRE_CORRECTION_ACTIVE_ARRAY_SIZE_MAXIMUM_RESOLUTION] else c[C.SENSOR_INFO_PRE_CORRECTION_ACTIVE_ARRAY_SIZE]
                val pixel = if (mode == "MaximumResolution") c[C.SENSOR_INFO_PIXEL_ARRAY_SIZE_MAXIMUM_RESOLUTION] else c[C.SENSOR_INFO_PIXEL_ARRAY_SIZE]
                // DngCreator requires a full sensor array. Smaller HAL RAW outputs
                // remain available as packed Raw16Le, not falsely advertised DNGs.
                val dng = (pre?.width() == size.width && pre.height() == size.height) ||
                    (pixel?.width == size.width && pixel.height == size.height)
                for (format in if(dng) listOf("Dng", "Raw16Le") else listOf("Raw16Le")) streams += stream(size.width, size.height, format, mode,
                    map.getOutputMinFrameDuration(ImageFormat.RAW_SENSOR, size), map.getOutputStallDuration(ImageFormat.RAW_SENSOR, size))
            }
            for (size in map.getOutputSizes(ImageFormat.JPEG).orEmpty()) streams += stream(size.width, size.height, "Jpeg", mode,
                map.getOutputMinFrameDuration(ImageFormat.JPEG, size), map.getOutputStallDuration(ImageFormat.JPEG, size))
        }
        collect(c[C.SCALER_STREAM_CONFIGURATION_MAP], "Default")
        collect(c[C.SCALER_STREAM_CONFIGURATION_MAP_MAXIMUM_RESOLUTION], "MaximumResolution")
        val previewMap = c[C.SCALER_STREAM_CONFIGURATION_MAP]
        val previews = previewMap?.getOutputSizes(ImageFormat.YUV_420_888).orEmpty()
            .filter { it.width <= 1280 && it.height <= 720 }.sortedBy { it.width.toLong() * it.height }
            .map { stream(it.width,it.height,"Rgb8","Default",previewMap!!.getOutputMinFrameDuration(ImageFormat.YUV_420_888,it),0) }
        val active = c[C.SENSOR_INFO_ACTIVE_ARRAY_SIZE]
        val zoom = c[C.CONTROL_ZOOM_RATIO_RANGE]
        val exposure = c[C.SENSOR_INFO_EXPOSURE_TIME_RANGE]
        val iso = c[C.SENSOR_INFO_SENSITIVITY_RANGE]
        val wbModes = c[C.CONTROL_AWB_AVAILABLE_MODES].orEmpty().mapNotNull { AWB[it] }.filter { it != "off" }
        val processing = JSONObject()
        fun modes(name: String, values: IntArray?) {
            processing.put(name, arr(values.orEmpty().mapNotNull { PROCESSING[it] }))
        }
        modes("edge", c[C.EDGE_AVAILABLE_EDGE_MODES])
        modes("noise_reduction", c[C.NOISE_REDUCTION_AVAILABLE_NOISE_REDUCTION_MODES])
        modes("hot_pixel", c[C.HOT_PIXEL_AVAILABLE_HOT_PIXEL_MODES])
        modes("shading", c[C.SHADING_AVAILABLE_MODES])
        modes("aberration", c[C.COLOR_CORRECTION_AVAILABLE_ABERRATION_MODES])
        modes("distortion", c[C.DISTORTION_CORRECTION_AVAILABLE_MODES])
        // Tonemap has a different enum (0 is contrast curve, not off).
        processing.put("tonemap", arr(c[C.TONEMAP_AVAILABLE_TONE_MAP_MODES].orEmpty().mapNotNull { TONEMAP[it] }))
        val logical = CameraMetadata.REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA in capabilities
        return obj(
            "schema_version" to 1, "camera_id" to id, "identity" to identity, "origin" to "Device",
            "lens_facing" to when(c[C.LENS_FACING]) { C.LENS_FACING_BACK -> "back"; C.LENS_FACING_FRONT -> "front"; else -> "external" },
            // Camera2 does not provide a trustworthy wide/tele marketing label.
            "lens_role" to null, "hardware_level" to c[C.INFO_SUPPORTED_HARDWARE_LEVEL]?.toString(),
            "logical" to logical,
            "physical_cameras" to arr(c.physicalCameraIds.map { obj("id" to it, "directly_openable" to (it in manager.cameraIdList), "routable" to false) }),
            "manual_sensor" to manual, "raw" to raw,
            "exposure_ns" to exposure?.let { range(it.lower,it.upper) },
            "sensitivity" to iso?.let { range(it.lower.toLong(),it.upper.toLong()) },
            "frame_duration_ns" to c[C.SENSOR_INFO_MAX_FRAME_DURATION]?.let { range(0,it) },
            "focus_millidiopters" to focus?.let { range(0,(it * 1000).toLong()) },
            "focus_calibration" to c[C.LENS_INFO_FOCUS_DISTANCE_CALIBRATION]?.toString(),
            "af_modes" to arr(af.filter { it == C.CONTROL_AF_MODE_OFF }.map { "off" }),
            "autofocus_center_supported" to (C.CONTROL_AF_MODE_AUTO in af && focus != null && (c[C.CONTROL_MAX_REGIONS_AF] ?: 0) > 0 && previews.any { it.getInt("width") <= 640 }),
            "focus_lock" to (C.CONTROL_AF_MODE_OFF in af && focus != null),
            "zoom_x1000" to zoom?.let { range(kotlin.math.ceil(it.lower * 1000.0).toLong(),kotlin.math.floor(it.upper * 1000.0).toLong()) },
            "active_array" to active?.let { obj("x" to it.left,"y" to it.top,"width" to it.width(),"height" to it.height()) },
            "crop_supported" to (active != null), "streams" to arr(streams), "preview_streams" to arr(previews),
            "wb_modes" to arr(wbModes), "wb_kelvin" to null, "wb_tint" to null, "wb_gain_x1000" to null,
            // JPEG output encoding: quality range is an API contract (1..100);
            // thumbnail sizes are real discovery from the HAL.
            "jpeg_quality" to range(1, 100),
            "jpeg_thumbnail_sizes" to arr(c[C.JPEG_AVAILABLE_THUMBNAIL_SIZES].orEmpty().map { obj("width" to it.width, "height" to it.height) }),
            "processing_modes" to processing,
            "ois_modes" to arr(c[C.LENS_INFO_AVAILABLE_OPTICAL_STABILIZATION].orEmpty().map { if(it == 0) "off" else "on" }),
            "eis_modes" to arr(c[C.CONTROL_AVAILABLE_VIDEO_STABILIZATION_MODES].orEmpty().filter { it in 0..1 }.map { if(it == 0) "off" else "on" })
        )
    }
    /** Extended diagnostic dump, retained independently of the Rust capability schema. */
    fun dump(): JSONObject = obj("schema_version" to 1, "identity" to identity,
        "cameras" to arr(manager.cameraIdList.map { id ->
            val c = characteristics(id)
            val keys = JSONObject()
            c.keys.forEach { key ->
                @Suppress("UNCHECKED_CAST")
                val value = c.get(key as C.Key<Any>)
                keys.put(key.name, describeValue(value))
            }
            obj("camera_id" to id,"characteristics" to keys,"capabilities" to describe(id),
                "request_controls" to arr(c.availableCaptureRequestKeys.orEmpty().map { key ->
                    val route = REQUEST_ROUTES[key.name]
                    obj("key" to key.name,"adapter_route" to route,"exposed" to (route != null))
                }),
                "available_result_keys" to arr(c.availableCaptureResultKeys.orEmpty().map { it.name }))
        }))
    private fun describeValue(value: Any?): Any = when(value) {
        null -> JSONObject.NULL
        is android.util.Rational -> obj("numerator" to value.numerator,"denominator" to value.denominator)
        is IntArray -> JSONArray(value.toList())
        is LongArray -> JSONArray(value.toList())
        is FloatArray -> JSONArray(value.map { describeValue(it) })
        is ByteArray -> JSONArray(value.map { it.toInt() and 255 })
        is Array<*> -> JSONArray(value.map { describeValue(it) })
        is Float -> if(value.isFinite()) value else value.toString()
        is Double -> if(value.isFinite()) value else value.toString()
        is Number, is Boolean, is String -> value
        else -> value.toString()
    }
    companion object {
        // Inventory is explicit: unknown/vendor keys remain visible as not exposed.
        // A route describes code support, not proof that the HAL honors a value.
        val REQUEST_ROUTES = mapOf(
            "android.sensor.exposureTime" to "settings.exposure_ns",
            "android.sensor.sensitivity" to "settings.sensitivity",
            "android.sensor.frameDuration" to "settings.frame_duration_ns",
            "android.sensor.pixelMode" to "settings.stream.pixel_mode",
            "android.lens.focusDistance" to "settings.focus.Manual.millidiopters",
            "android.control.afMode" to "manual off; autofocus_center operation uses auto",
            "android.control.afTrigger" to "autofocus_center operation",
            "android.control.afRegions" to "autofocus_center operation (central region)",
            "android.control.aeMode" to "fixed off for acquisition; on during autofocus",
            "android.control.mode" to "adapter-managed AUTO",
            "android.control.captureIntent" to "adapter-managed preview/still intent",
            "android.control.zoomRatio" to "settings.zoom_x1000",
            "android.scaler.cropRegion" to "settings.crop",
            "android.control.awbMode" to "settings.white_balance.Mode",
            "android.colorCorrection.mode" to "manual WB transform mode, when advertised",
            "android.colorCorrection.gains" to "settings.white_balance.Manual.gains_x1000, when advertised",
            "android.colorCorrection.transform" to "settings.white_balance.Manual.transform_millionths, when advertised",
            "android.edge.mode" to "settings.processing.edge",
            "android.noiseReduction.mode" to "settings.processing.noise_reduction",
            "android.hotPixel.mode" to "settings.processing.hot_pixel",
            "android.shading.mode" to "settings.processing.shading",
            "android.colorCorrection.aberrationMode" to "settings.processing.aberration",
            "android.distortionCorrection.mode" to "settings.processing.distortion",
            "android.tonemap.mode" to "settings.processing.tonemap (fast/high_quality)",
            "android.lens.opticalStabilizationMode" to "settings.ois",
            "android.control.videoStabilizationMode" to "settings.eis",
            "android.statistics.lensShadingMapMode" to "adapter requests ON when advertised"
        )
        fun range(min: Long,max: Long) = obj("min" to min,"max" to max)
        fun stream(w: Int,h: Int,format: String,mode: String,min: Long,stall: Long) =
            obj("width" to w,"height" to h,"format" to format,"pixel_mode" to mode,"binned" to null,
                "min_frame_duration_ns" to min,"stall_duration_ns" to stall)
        val AWB = mapOf(0 to "off",1 to "auto",2 to "incandescent",3 to "fluorescent",4 to "warm_fluorescent",5 to "daylight",6 to "cloudy_daylight",7 to "twilight",8 to "shade")
        val PROCESSING = mapOf(0 to "off",1 to "fast",2 to "high_quality",3 to "minimal",4 to "zero_shutter_lag")
        val TONEMAP = mapOf(1 to "fast",2 to "high_quality")
    }
}
