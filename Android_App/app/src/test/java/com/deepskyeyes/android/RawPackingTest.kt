package com.deepskyeyes.android

import com.deepskyeyes.android.camera.RawPacking
import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

class RawPackingTest {
    @Test fun preservesAllPixelsAndSkipsOnlyRowPadding() {
        val source=ByteBuffer.wrap(byteArrayOf(1,2,3,4,99,99,5,6,7,8))
        assertArrayEquals(byteArrayOf(1,2,3,4,5,6,7,8),RawPacking.pack(source,2,2,6,2))
        assertEquals(0,source.position())
    }
    @Test fun supportsPixelPaddingAndNonzeroBufferPosition() {
        val source=ByteBuffer.wrap(byteArrayOf(99,1,2,99,3,4,99,5,6,99,7,8))
        source.position(1)
        assertArrayEquals(byteArrayOf(1,2,3,4,5,6,7,8),RawPacking.pack(source,2,2,6,3))
        assertEquals(1,source.position())
    }
    @Test fun refusesTruncationInsteadOfZeroFilling() {
        assertThrows(IllegalArgumentException::class.java) { RawPacking.pack(ByteBuffer.allocate(7),2,2,4,2) }
        assertThrows(IllegalArgumentException::class.java) { RawPacking.pack(ByteBuffer.allocate(8),2,2,3,2) }
    }
}
