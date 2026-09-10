//! Frame RAW_SENSOR 16-bit.
#[derive(Debug, Clone)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u16>,
}

impl RawFrame {
    pub fn validate(&self) -> std::io::Result<()> {
        let pixels = (self.width as usize).checked_mul(self.height as usize);
        if self.width == 0 || self.height == 0 || pixels != Some(self.data.len()) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "RAW dimensions do not match sample count"));
        }
        Ok(())
    }
    /// Row-major unsigned 16-bit little-endian samples, NOT DNG. Save as .raw16.
    pub fn to_le_bytes(&self) -> std::io::Result<Vec<u8>> {
        self.validate()?;
        let length = self.data.len().checked_mul(2).ok_or_else(|| std::io::Error::other("RAW size overflow"))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(std::io::Error::other)?;
        for sample in &self.data { bytes.extend_from_slice(&sample.to_le_bytes()); }
        Ok(bytes)
    }
    /// Unpack RAW_SENSOR rows, discarding only row padding.
    pub fn from_le_bytes(width: u32, height: u32, row_stride: usize, bytes: &[u8]) -> std::io::Result<Self> {
        let row = (width as usize).checked_mul(2).ok_or_else(|| std::io::Error::other("row overflow"))?;
        if width == 0 || height == 0 || row_stride < row || row_stride.checked_mul(height as usize) != Some(bytes.len()) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid RAW_SENSOR stride or byte count"));
        }
        let mut data = Vec::new();
        data.try_reserve_exact(row / 2 * height as usize).map_err(std::io::Error::other)?;
        for bytes in bytes.chunks_exact(row_stride) {
            data.extend(bytes[..row].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])));
        }
        Ok(Self { width, height, data })
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn padded_rows_roundtrip() {
        let frame = RawFrame::from_le_bytes(2, 2, 6, &[1,0,255,255,9,9,0,128,2,0,9,9]).unwrap();
        assert_eq!(frame.data, [1,65535,32768,2]);
        assert_eq!(frame.to_le_bytes().unwrap(), [1,0,255,255,0,128,2,0]);
        assert!(RawFrame::from_le_bytes(2, 2, 3, &[0;6]).is_err());
        assert!(RawFrame { width: 2, height: 2, data: vec![1] }.to_le_bytes().is_err());
    }
}
