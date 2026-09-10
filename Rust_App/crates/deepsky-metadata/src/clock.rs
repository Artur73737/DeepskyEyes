//! Clock: Android vs PC non sincronizzati (README §71).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClockOffset {
    pub offset_ns: i64,
}
