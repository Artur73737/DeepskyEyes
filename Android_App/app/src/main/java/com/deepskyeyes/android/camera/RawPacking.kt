package com.deepskyeyes.android.camera

import java.nio.ByteBuffer

/** Preserve every sensor pixel byte-for-byte; remove only row/pixel padding. */
object RawPacking {
    fun pack(source: ByteBuffer,width: Int,height: Int,rowStride: Int,pixelStride: Int): ByteArray {
        require(width > 0 && height > 0 && pixelStride >= 2)
        val rowBytes = Math.addExact(Math.multiplyExact(width-1,pixelStride),2)
        require(rowStride >= rowBytes)
        val required = Math.addExact(Math.multiplyExact(height-1,rowStride),rowBytes)
        val buffer = source.duplicate()
        require(buffer.remaining() >= required) { "Truncated RAW plane" }
        val base = buffer.position()
        val output = ByteArray(Math.multiplyExact(Math.multiplyExact(width,height),2))
        val row = if(pixelStride == 2) null else ByteArray(rowBytes)
        for(y in 0 until height) {
            buffer.position(base+y*rowStride)
            if(row == null) buffer.get(output,y*width*2,width*2)
            else {
                buffer.get(row)
                for(x in 0 until width) {
                    output[(y*width+x)*2]=row[x*pixelStride]
                    output[(y*width+x)*2+1]=row[x*pixelStride+1]
                }
            }
        }
        return output
    }
}
