//! Istogramma RGB/luminanza + clipping.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub r: [u32; 256],
    pub g: [u32; 256],
    pub b: [u32; 256],
}

impl Default for Histogram {
    fn default() -> Self {
        Self { r: [0; 256], g: [0; 256], b: [0; 256] }
    }
}
