//! Metriche RX/TX, drop, timing.
#[derive(Debug, Default, Clone, Copy)]
pub struct Metrics {
    pub rx_mbps_x100: u64,
    pub dropped_raw: u64,
    pub dropped_preview: u64,
}
