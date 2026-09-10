//! Frame acquisito (riferimento, non payload intero in memoria).
#[derive(Debug, Clone)]
pub struct AcquiredFrame {
    pub frame_id: String,
    pub request_id: u32,
    pub size_bytes: u64,
}
