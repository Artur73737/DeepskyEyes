//! Piano sequenza.
#[derive(Debug, Clone)]
pub struct SequencePlan {
    pub frames: u32,
    pub exposure_ns: u64,
    pub sensitivity: u32,
    pub delay_ns: u64,
}

impl SequencePlan {
    pub fn lights_300x15s(sensitivity: u32) -> Self {
        Self { frames: 300, exposure_ns: 15_000_000_000, sensitivity, delay_ns: 1_000_000_000 }
    }
}
