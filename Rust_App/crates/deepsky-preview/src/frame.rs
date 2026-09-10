//! Frame preview (YUV/etc, disposable).
#[derive(Debug, Clone)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl PreviewFrame {
    /// Packed RGB8, independent of RAW resolution and timing.
    pub fn validate(&self) -> Result<(), &'static str> {
        let expected = (self.width as usize).checked_mul(self.height as usize).and_then(|v| v.checked_mul(3));
        if self.width == 0 || self.height == 0 || expected != Some(self.data.len()) { return Err("RGB dimensions do not match byte count"); }
        Ok(())
    }
}
