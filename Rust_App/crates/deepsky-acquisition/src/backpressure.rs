//! Backpressure: BLOCK o DROP PREVIEW, mai drop RAW silenzioso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressurePolicy {
    Block,
    DropPreview,
}
