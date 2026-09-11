package com.deepskyeyes.android.transport

import com.deepskyeyes.android.camera.CameraEngine
import com.deepskyeyes.android.protocol.*
import com.deepskyeyes.android.service.BridgeStatus
import org.json.JSONObject
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/** Loopback-only development server, reached over an authorized ADB forward. */
class BridgeServer(private val engine: CameraEngine, private val thermal: () -> JSONObject) : AutoCloseable {
    @Volatile private var running = true
    @Volatile private var listener: ServerSocket? = null
    @Volatile private var client: Socket? = null
    private val watchdog = Executors.newSingleThreadScheduledExecutor { task -> Thread(task,"Bridge-watchdog").apply { isDaemon = true } }
    fun run() {
        try {
            val server = ServerSocket().apply { reuseAddress = true; bind(java.net.InetSocketAddress(InetAddress.getByName("127.0.0.1"),7878),1) }
            listener = server
            if (!running) { server.close(); return }
            BridgeStatus.update { it.copy(running = true) }
            BridgeStatus.log("Listening on 127.0.0.1:7878 · ADB")
            while(running) {
                val socket = server.accept()
                client = socket
                socket.use {
                    try { handle(it) }
                    catch(e: Exception) { if(running) BridgeStatus.log("Connection ended: ${e.message}") }
                    finally { engine.closeCamera(); BridgeStatus.update { state -> state.copy(connected = false) } }
                }
                client = null
            }
        } catch(e: Exception) { if(running) BridgeStatus.log("Server error: ${e.message}") }
        finally { listener?.close(); watchdog.shutdownNow(); BridgeStatus.update { it.copy(running = false, connected = false) } }
    }
    private fun handle(socket: Socket) {
        requireCamera(!BridgeStatus.state.value.busy,"InvalidState","Local acquisition in progress")
        socket.soTimeout = 120_000; socket.tcpNoDelay = true
        val input = socket.getInputStream().buffered(64*1024)
        val output = socket.getOutputStream().buffered(64*1024)
        var hello = false; var incoming = 0L; var outgoing = 0L
        BridgeStatus.update { it.copy(connected = true) }
        while(running) {
            val frame = FrameCodec.read(input)
            require(frame.type == 1 && frame.sequence > incoming && frame.requestId > 0) { "Invalid request type or sequence" }
            incoming = frame.sequence; outgoing++
            BridgeStatus.update { it.copy(rxBytes = it.rxBytes + frame.payload.size + 58) }
            // Abort writes to a peer which stops reading, including during a large RAW transfer.
            val timeout = watchdog.schedule({ runCatching { socket.close() }; engine.abort() },120,TimeUnit.SECONDS)
            try {
                val request = JSONObject(String(frame.payload,Charsets.UTF_8))
                requireCamera(request.keys().asSequence().all { it in setOf("method","params") }, "InvalidRequest","Unknown request field")
                val method = request.getString("method")
                val params = request.optJSONObject("params") ?: JSONObject()
                BridgeStatus.log("#${frame.requestId} $method")
                if (method != "hello" && !hello) fault("InvalidState","HELLO required")
                if(method == "capture" || method == "preview_binary") {
                    val capture = if(method == "capture") engine.capture() else engine.previewFrame()
                    val metadata = capture.metadata.toString().toByteArray(Charsets.UTF_8)
                    val length = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(metadata.size).array()
                    FrameCodec.write(output,if(method == "capture") 4 else 5,frame.requestId,outgoing,length,metadata,capture.bytes)
                    BridgeStatus.update { it.copy(captured = it.captured + if(method == "capture") 1 else 0,txBytes = it.txBytes+58+4+metadata.size+capture.bytes.size) }
                    continue
                }
                val result: Any? = when(method) {
                    "hello" -> {
                        requireCamera(params.optInt("protocol_version") == 1,"Unsupported","Protocol version must be 1")
                        hello = true; obj("protocol_version" to 1,"identity" to engine.identity,"binary_preview" to true)
                    }
                    "discover" -> engine.discovery.discover()
                    "open" -> { engine.open(params); null }
                    "configure" -> engine.configure(params.getJSONObject("request"),params.getString("policy"))
                    "preview" -> engine.preview()
                    "autofocus_center" -> engine.autofocusCenter()
                    "thermal" -> thermal()
                    "close" -> { engine.closeCamera(); null }
                    "ping" -> obj("echo" to request.opt("params"),"server_time_ns" to System.currentTimeMillis()*1_000_000L,
                        "elapsed_realtime_ns" to android.os.SystemClock.elapsedRealtimeNanos())
                    "capability_dump" -> engine.discovery.dump()
                    "status" -> obj("camera_state" to BridgeStatus.state.value.cameraState,"captured" to BridgeStatus.state.value.captured)
                    else -> fault("Unsupported","Unknown method: $method")
                }
                val data = obj("status" to "ok","result" to result).toString().toByteArray(Charsets.UTF_8)
                FrameCodec.write(output,2,frame.requestId,outgoing,data)
                BridgeStatus.update { it.copy(txBytes = it.txBytes+data.size+58) }
            } catch(e: Exception) {
                if(socket.isClosed) throw e
                val code = (e as? CameraFault)?.code ?: if(e is IllegalArgumentException || e is org.json.JSONException) "InvalidRequest" else "Io"
                val message = e.message ?: e.javaClass.simpleName
                BridgeStatus.log("$code · $message")
                val data = obj("status" to "error","code" to code,"message" to message).toString().toByteArray(Charsets.UTF_8)
                FrameCodec.write(output,2,frame.requestId,outgoing,data)
            } finally { timeout.cancel(false) }
        }
    }
    override fun close() {
        running = false
        runCatching { listener?.close() }; runCatching { client?.close() }
        engine.abort(); watchdog.shutdownNow()
    }
}
