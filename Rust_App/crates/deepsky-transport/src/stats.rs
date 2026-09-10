//! Statistiche RX/TX per diagnostics panel.
#[derive(Debug, Default, Clone, Copy)]
pub struct TransportStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_mbps_x100: u64,
}
