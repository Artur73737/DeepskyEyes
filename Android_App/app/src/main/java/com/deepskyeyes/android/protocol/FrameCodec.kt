package com.deepskyeyes.android.protocol

import java.io.InputStream
import java.io.OutputStream
import java.io.EOFException
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.MessageDigest

/** Byte-exact Rust framing: DSKY + u16 version/type/flags + u32 id/len + u64 sequence. */
object FrameCodec {
    const val HEADER_SIZE = 26
    const val MAX_PAYLOAD = 256 * 1024 * 1024
    const val MAX_REQUEST = 1024 * 1024
    data class Frame(val type: Int, val requestId: Long, val sequence: Long, val payload: ByteArray)
    fun read(input: InputStream, limit: Int = MAX_REQUEST): Frame {
        val h = exact(input, HEADER_SIZE)
        val b = ByteBuffer.wrap(h).order(ByteOrder.LITTLE_ENDIAN)
        require(ByteArray(4).also(b::get).contentEquals("DSKY".toByteArray())) { "Bad magic" }
        require(b.short.toInt() == 1) { "Unsupported protocol version" }
        val type = b.short.toInt() and 65535
        require(b.short.toInt() == 0) { "Unsupported flags" }
        val id = b.int.toLong() and 0xffffffffL
        val length = b.int.toLong() and 0xffffffffL
        val seq = b.long
        require(seq > 0 && length <= limit && length <= MAX_PAYLOAD) { "Invalid sequence or payload size" }
        val payload = exact(input, length.toInt())
        val checksum = exact(input, 32)
        val digest = MessageDigest.getInstance("SHA-256").apply { update(h); update(payload) }.digest()
        require(MessageDigest.isEqual(checksum, digest)) { "Checksum mismatch" }
        return Frame(type, id, seq, payload)
    }
    fun write(output: OutputStream, type: Int, id: Long, sequence: Long, vararg parts: ByteArray) {
        val length = parts.sumOf { it.size.toLong() }
        require(length <= MAX_PAYLOAD && id in 0..0xffffffffL && sequence > 0)
        val header = ByteBuffer.allocate(HEADER_SIZE).order(ByteOrder.LITTLE_ENDIAN)
            .put("DSKY".toByteArray()).putShort(1).putShort(type.toShort()).putShort(0)
            .putInt(id.toInt()).putInt(length.toInt()).putLong(sequence).array()
        val digest = MessageDigest.getInstance("SHA-256")
        output.write(header); digest.update(header)
        parts.forEach { output.write(it); digest.update(it) }
        output.write(digest.digest()); output.flush()
    }
    private fun exact(input: InputStream, size: Int): ByteArray {
        val bytes = ByteArray(size)
        var offset = 0
        while (offset < size) {
            val n = input.read(bytes, offset, size - offset)
            if (n < 0) throw EOFException("Disconnected inside frame")
            if (n == 0) continue
            offset += n
        }
        return bytes
    }
}
