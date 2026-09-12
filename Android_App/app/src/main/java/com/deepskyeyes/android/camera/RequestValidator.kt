package com.deepskyeyes.android.camera

import com.deepskyeyes.android.protocol.*
import org.json.JSONObject
import org.json.JSONArray

/** Pure validation against the advertised snapshot. Never infer an unsupported control. */
object RequestValidator {
    fun validate(caps: JSONObject, request: JSONObject, policy: String): JSONObject {
        requireCamera(policy in listOf("Reject", "Clamp"), "InvalidRequest", "Unknown validation policy")
        requireCamera(request.getLong("request_id") >= 0, "InvalidRequest", "Invalid request ID")
        val selection = request.getJSONObject("selection")
        requireCamera(selection.getString("camera_id") == caps.getString("camera_id"), "Unsupported", "Camera ID mismatch")
        requireCamera(selection.isNull("physical_id"), "Unsupported", "Physical routing requires a validated physical stream session")
        val applied = request.copyJson()
        val settings = applied.getJSONObject("settings")
        val changes = JSONArray()
        val allowed = setOf("exposure_ns","sensitivity","frame_duration_ns","focus","zoom_x1000","crop","stream","white_balance","processing","ois","eis","jpeg")
        requireCamera(settings.keys().asSequence().all { it in allowed }, "Unsupported", "Unknown camera control")
        if (listOf("exposure_ns","sensitivity","frame_duration_ns").any { !settings.isNull(it) }) {
            requireCamera(caps.optBoolean("manual_sensor"), "Unsupported", "MANUAL_SENSOR / AE OFF unavailable")
        }
        fun numeric(container: JSONObject, key: String, rangeKey: String = key, field: String = key) {
            if (container.isNull(key)) return
            val range = caps.optJSONObject(rangeKey) ?: fault("Unsupported", field)
            val v = container.getLong(key)
            val min = range.getLong("min"); val max = range.getLong("max")
            requireCamera(min >= 0 && max >= min, "InvalidCapabilities", "Invalid $rangeKey range")
            val accepted = v.coerceIn(min, max)
            if (accepted != v) {
                requireCamera(policy == "Clamp", "OutOfRange", field)
                container.put(key, accepted)
                changes.put(obj("field" to field, "requested" to v, "applied" to accepted))
            }
        }
        for (key in listOf("exposure_ns","sensitivity","frame_duration_ns","zoom_x1000")) numeric(settings, key)
        val stream = settings.optJSONObject("stream") ?: fault("InvalidRequest", "Explicit stream required")
        val supported = caps.getJSONArray("streams").objects().any { it.sameJson(stream) }
        requireCamera(supported && stream.getInt("width") > 0 && stream.getInt("height") > 0, "Unsupported", "Stream format, dimensions or sensor pixel mode not announced")
        val exposure = settings.longOrNull("exposure_ns") ?: fault("InvalidRequest", "Explicit exposure required")
        requireCamera(!settings.isNull("sensitivity"), "InvalidRequest", "Explicit sensitivity required")
        val minimum = maxOf(exposure, stream.longOrNull("min_frame_duration_ns") ?: 0)
        val duration = settings.longOrNull("frame_duration_ns")
        requireCamera(duration == null || duration >= minimum, "InvalidRequest", "Frame duration shorter than exposure/stream minimum")
        caps.optJSONObject("frame_duration_ns")?.let { requireCamera(minimum <= it.getLong("max"), "OutOfRange", "Frame period exceeds hardware maximum") }
        fun mode(value: String, key: String) {
            requireCamera(caps.optJSONArray(key)?.strings()?.contains(value) == true, "Unsupported", "$key: $value")
        }
        settings.optJSONObject("focus")?.let { focus ->
            val manual = focus.optJSONObject("Manual") ?: fault("Unsupported", "Only explicit manual focus is implemented; autofocus lock is not fabricated")
            mode("off", "af_modes")
            numeric(manual, "millidiopters", "focus_millidiopters", "focus_millidiopters")
            if (manual.optBoolean("locked")) requireCamera(caps.optBoolean("focus_lock"), "Unsupported", "Focus lock")
        }
        settings.optJSONObject("white_balance")?.let { wb ->
            when {
                wb.has("Mode") -> { val m = wb.getString("Mode"); requireCamera(m !in listOf("manual","temperature"), "InvalidRequest", "WB needs parameters"); mode(m, "wb_modes") }
                wb.has("Manual") -> {
                    mode("manual", "wb_modes")
                    val manual = wb.getJSONObject("Manual")
                    val gains = manual.getJSONArray("gains_x1000")
                    val matrix = manual.getJSONArray("transform_millionths")
                    requireCamera(gains.length() == 4 && matrix.length() == 9, "InvalidRequest", "WB gains/matrix dimensions")
                    for (i in 0..3) {
                        val box = obj("gain" to gains.getLong(i))
                        numeric(box, "gain", "wb_gain_x1000", "wb_gain_$i")
                        gains.put(i, box.getLong("gain"))
                    }
                    for (i in 0..8) requireCamera(matrix.getLong(i) in Int.MIN_VALUE.toLong()..Int.MAX_VALUE.toLong(), "OutOfRange", "WB matrix")
                }
                else -> fault("Unsupported", "WB temperature/tint not advertised by this adapter")
            }
        }
        settings.optJSONObject("crop")?.let { crop ->
            requireCamera(caps.optBoolean("crop_supported"), "Unsupported", "Crop")
            val area = caps.getJSONObject("active_array")
            for (key in listOf("x","y","width","height")) requireCamera(crop.getLong(key) in 0..Int.MAX_VALUE.toLong(), "OutOfRange", "Crop $key")
            requireCamera(crop.getLong("width") > 0 && crop.getLong("height") > 0 && crop.getLong("x") >= area.getLong("x") && crop.getLong("y") >= area.getLong("y") &&
                crop.getLong("x")+crop.getLong("width") <= area.getLong("x")+area.getLong("width") &&
                crop.getLong("y")+crop.getLong("height") <= area.getLong("y")+area.getLong("height"), "OutOfRange", "Crop outside active array")
        }
        val modes = settings.optJSONObject("processing") ?: JSONObject()
        modes.keys().forEach { key ->
            requireCamera(caps.getJSONObject("processing_modes").optJSONArray(key)?.strings()?.contains(modes.getString(key)) == true, "Unsupported", "Processing $key")
        }
        settings.stringOrNull("ois")?.let { mode(it, "ois_modes") }
        settings.stringOrNull("eis")?.let { mode(it, "eis_modes") }
        settings.optJSONObject("jpeg")?.let { jpeg ->
            // JPEG controls are encoder settings: on a non-JPEG stream the HAL
            // would silently ignore them, so refuse instead of pretending.
            val active = listOf("quality","orientation","thumbnail_quality","thumbnail_size").any { !jpeg.isNull(it) }
            if (active) requireCamera(stream.getString("format") == "Jpeg", "InvalidRequest", "JPEG controls require a JPEG stream")
            if (!jpeg.isNull("quality")) {
                val range = caps.optJSONObject("jpeg_quality") ?: fault("Unsupported", "jpeg quality not announced")
                val q = jpeg.getLong("quality")
                requireCamera(q in range.getLong("min")..range.getLong("max"), "OutOfRange", "jpeg_quality")
            }
            if (!jpeg.isNull("thumbnail_quality")) {
                val range = caps.optJSONObject("jpeg_quality") ?: fault("Unsupported", "jpeg quality not announced")
                val q = jpeg.getLong("thumbnail_quality")
                requireCamera(q in range.getLong("min")..range.getLong("max"), "OutOfRange", "jpeg_thumbnail_quality")
            }
            if (!jpeg.isNull("orientation")) {
                requireCamera(jpeg.getLong("orientation") in listOf(0L, 90L, 180L, 270L), "InvalidRequest", "jpeg_orientation")
            }
            jpeg.optJSONObject("thumbnail_size")?.let { size ->
                val announced = caps.optJSONArray("jpeg_thumbnail_sizes")?.objects().orEmpty()
                requireCamera(announced.any { it.getInt("width") == size.getInt("width") && it.getInt("height") == size.getInt("height") },
                    "Unsupported", "jpeg thumbnail size not announced")
            }
        }
        return obj("requested" to request.copyJson(), "applied" to applied, "adjustments" to changes)
    }
}
