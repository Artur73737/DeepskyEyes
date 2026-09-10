//! Statistiche preview: mean, frame n, dropped.
#[derive(Debug, Default, Clone, Copy)]
pub struct PreviewStats {
    pub frame_number: u64,
    pub dropped: u64,
}
