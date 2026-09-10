//! Processing: EDGE/NOISE/HOTPIXEL/SHADING/TONEMAP (README §19).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessingModes {
    pub edge: u8,
    pub noise_reduction: u8,
    pub hot_pixel: u8,
    pub shading: u8,
    pub tonemap: u8,
}
