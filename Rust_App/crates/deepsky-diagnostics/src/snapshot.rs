//! Snapshot diagnostico per pagina Diagnostics.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticSnapshot {
    pub camera_state: String,
    pub exposure_requested_ns: u64,
    pub exposure_applied_ns: u64,
}
