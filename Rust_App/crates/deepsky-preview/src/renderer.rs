//! Renderer preview — adapter verso GPUI (tipi GPUI isolati in deepsky-ui).
pub struct PreviewRenderer {
    pub zoom: f32,
}

impl Default for PreviewRenderer {
    fn default() -> Self {
        Self { zoom: 1.0 }
    }
}
