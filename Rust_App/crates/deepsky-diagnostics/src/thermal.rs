//! Stato termico NORMAL/WARM/HOT/THROTTLING/CRITICAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThermalState {
    #[default]
    Normal,
    Warm,
    Hot,
    Throttling,
    Critical,
}
