//! Metadati per frame (README §72).
#[derive(Debug, Clone, Default)]
pub struct FrameMetadata {
    pub frame_id: String,
    pub camera_id: String,
    pub exposure_requested_ns: u64,
    pub exposure_reported_ns: Option<u64>,
    pub sensitivity_requested: u32,
    pub sensitivity_reported: Option<u32>,
    pub timestamp_ns: u64,
}
