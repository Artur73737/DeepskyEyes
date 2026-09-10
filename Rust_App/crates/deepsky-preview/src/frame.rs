//! Frame preview (YUV/etc, disposable).
#[derive(Debug, Clone)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
