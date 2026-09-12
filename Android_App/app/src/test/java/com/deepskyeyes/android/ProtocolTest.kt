package com.deepskyeyes.android

import com.deepskyeyes.android.protocol.*
import com.deepskyeyes.android.camera.RequestValidator
import org.json.JSONObject
import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.MessageDigest

class ProtocolTest {
    private fun encoded(): ByteArray = ByteArrayOutputStream().also {
        FrameCodec.write(it,1,0xfedcba98L,42,"{\"method\":\"hello\"}".toByteArray())
    }.toByteArray()
    @Test fun rustHeaderLayoutAndUnsignedId() {
        val bytes = encoded()
        assertEquals("44534b5901000100000098badcfe120000002a00000000000000",bytes.take(26).joinToString("") { "%02x".format(it.toInt() and 255) })
        val frame = FrameCodec.read(ByteArrayInputStream(bytes))
        assertEquals(0xfedcba98L,frame.requestId)
        assertEquals(42L,frame.sequence)
        assertEquals("{\"method\":\"hello\"}",String(frame.payload))
    }
    @Test fun fragmentedInput() {
        val source = ByteArrayInputStream(encoded())
        val fragmented = object : InputStream() {
            override fun read() = source.read()
            override fun read(b: ByteArray,off: Int,len: Int) = source.read(b,off,minOf(len,3))
        }
        assertEquals(42L,FrameCodec.read(fragmented).sequence)
    }
    @Test fun allTruncationsRejected() {
        val frame = encoded()
        for(size in frame.indices) assertThrows(Exception::class.java) { FrameCodec.read(ByteArrayInputStream(frame.copyOf(size))) }
    }
    @Test fun tamperingRejected() {
        val bytes = encoded(); bytes[26] = (bytes[26].toInt() xor 1).toByte()
        assertThrows(IllegalArgumentException::class.java) { FrameCodec.read(ByteArrayInputStream(bytes)) }
    }
    @Test fun oversizedRequestRejectedBeforeAllocation() {
        val h = encoded().copyOf(26)
        ByteBuffer.wrap(h).order(ByteOrder.LITTLE_ENDIAN).putInt(14,Int.MAX_VALUE)
        assertThrows(IllegalArgumentException::class.java) { FrameCodec.read(ByteArrayInputStream(h)) }
    }
    @Test fun binaryCaptureCanBeDecodedByRustFraming() {
        val metadata = "{\"origin\":\"Device\"}".toByteArray()
        val prefix = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(metadata.size).array()
        val raw = byteArrayOf(0,1,2,3)
        val output = ByteArrayOutputStream()
        FrameCodec.write(output,4,7,2,prefix,metadata,raw)
        val frame = FrameCodec.read(ByteArrayInputStream(output.toByteArray()))
        assertEquals(4,frame.type)
        assertEquals(metadata.size,ByteBuffer.wrap(frame.payload).order(ByteOrder.LITTLE_ENDIAN).int)
        assertArrayEquals(raw,frame.payload.takeLast(4).toByteArray())
    }
}

class RequestValidatorTest {
    private fun stream() = obj("width" to 4,"height" to 2,"format" to "Raw16Le","pixel_mode" to "Default","binned" to null,"min_frame_duration_ns" to 10,"stall_duration_ns" to 0)
    private fun caps() = obj("camera_id" to "measured-id","manual_sensor" to true,"exposure_ns" to obj("min" to 10,"max" to 100),
        "sensitivity" to obj("min" to 100,"max" to 800),"frame_duration_ns" to obj("min" to 10,"max" to 200),
        "streams" to JSONArray().put(stream()),"focus_millidiopters" to obj("min" to 0,"max" to 2000),
        "focus_lock" to true,"af_modes" to JSONArray().put("off"),"wb_modes" to JSONArray().put("daylight"),
        "processing_modes" to JSONObject(),"ois_modes" to JSONArray().put("off"),"eis_modes" to JSONArray().put("off"))
    private fun request() = obj("request_id" to 5,"selection" to obj("camera_id" to "measured-id","physical_id" to null),
        "settings" to obj("exposure_ns" to 50,"sensitivity" to 200,"stream" to stream(),
            "focus" to obj("Manual" to obj("millidiopters" to 0,"locked" to true)),"white_balance" to obj("Mode" to "daylight"),"processing" to JSONObject()))
    @Test fun validConfigurationPreservesRequested() {
        val request = request()
        val result = RequestValidator.validate(caps(),request,"Reject")
        assertTrue(result.getJSONObject("requested").sameJson(request))
        assertTrue(result.getJSONObject("applied").sameJson(request))
        assertEquals(0,result.getJSONArray("adjustments").length())
    }
    @Test fun clampingIsExplicitAndDoesNotMutateRequest() {
        val request=request(); request.getJSONObject("settings").put("exposure_ns",120)
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps(),request,"Reject") }
        val result=RequestValidator.validate(caps(),request,"Clamp")
        assertEquals(120,request.getJSONObject("settings").getInt("exposure_ns"))
        assertEquals(100,result.getJSONObject("applied").getJSONObject("settings").getInt("exposure_ns"))
        assertEquals("exposure_ns",result.getJSONArray("adjustments").getJSONObject(0).getString("field"))
    }
    @Test fun invalidStreamIsNotSubstituted() {
        val request=request(); request.getJSONObject("settings").getJSONObject("stream").put("width",50)
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps(),request,"Clamp") }
    }
    @Test fun unknownCapabilityIsNotSupported() {
        val caps=caps(); caps.remove("focus_millidiopters")
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps,request(),"Reject") }
    }
    @Test fun frameDurationCannotBeShorterThanExposure() {
        val request=request(); request.getJSONObject("settings").put("frame_duration_ns",20)
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps(),request,"Clamp") }
    }
    @Test fun physicalRoutingAndUnknownControlsRejected() {
        val request=request(); request.getJSONObject("selection").put("physical_id","unmeasured")
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps(),request,"Reject") }
        val unknown=request(); unknown.getJSONObject("settings").put("magic_denoise",true)
        assertThrows(CameraFault::class.java) { RequestValidator.validate(caps(),unknown,"Reject") }
    }
    private fun jpegCaps() = caps().apply {
        put("jpeg_quality", obj("min" to 1, "max" to 100))
        put("jpeg_thumbnail_sizes", JSONArray().put(obj("width" to 0, "height" to 0)).put(obj("width" to 320, "height" to 240)))
        getJSONArray("streams").put(obj("width" to 8, "height" to 6, "format" to "Jpeg", "pixel_mode" to "Default", "binned" to null, "min_frame_duration_ns" to 10, "stall_duration_ns" to 0))
    }
    private fun jpegRequest(format: String, jpeg: JSONObject?) = request().apply {
        val stream = getJSONObject("settings").getJSONObject("stream")
        if (format == "Jpeg") { stream.put("width", 8); stream.put("height", 6); stream.put("format", "Jpeg") }
        getJSONObject("settings").put("jpeg", jpeg ?: JSONObject.NULL)
    }
    private fun jpegCode(format: String, jpeg: JSONObject?): String? = try {
        RequestValidator.validate(jpegCaps(), jpegRequest(format, jpeg), "Reject"); null
    } catch (e: CameraFault) { e.code }
    @Test fun jpegQualityLimits() {
        for (q in listOf(1, 100)) assertNull(jpegCode("Jpeg", obj("quality" to q)))
        for (q in listOf(0, 101)) assertEquals("OutOfRange", jpegCode("Jpeg", obj("quality" to q)))
        assertEquals("OutOfRange", jpegCode("Jpeg", obj("thumbnail_quality" to 0)))
    }
    @Test fun jpegOrientationSet() {
        for (o in listOf(0, 90, 180, 270)) assertNull(jpegCode("Jpeg", obj("orientation" to o)))
        assertEquals("InvalidRequest", jpegCode("Jpeg", obj("orientation" to 45)))
    }
    @Test fun jpegThumbnailSizeAnnounced() {
        assertNull(jpegCode("Jpeg", obj("thumbnail_size" to obj("width" to 320, "height" to 240))))
        assertNull(jpegCode("Jpeg", obj("thumbnail_size" to obj("width" to 0, "height" to 0))))
        assertEquals("Unsupported", jpegCode("Jpeg", obj("thumbnail_size" to obj("width" to 640, "height" to 480))))
    }
    @Test fun jpegRefusedOnRaw() {
        assertEquals("InvalidRequest", jpegCode("Raw16Le", obj("quality" to 90)))
        assertNull(jpegCode("Raw16Le", obj()))
    }
    @Test fun legacyRequestWithoutJpegKey() {
        val req = jpegRequest("Jpeg", null)
        req.getJSONObject("settings").remove("jpeg")
        RequestValidator.validate(jpegCaps(), req, "Reject")
    }
}
