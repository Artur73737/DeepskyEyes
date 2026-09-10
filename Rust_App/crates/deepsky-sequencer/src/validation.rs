//! Validazione pre-start (README §36).
#[derive(Debug, Clone, Default)]
pub struct SequenceValidation {
    pub camera_connected: bool,
    pub raw_supported: bool,
    pub focus_locked: bool,
    pub storage_ok: bool,
    pub thermal_ok: bool,
}

impl SequenceValidation {
    pub fn ok(&self) -> bool {
        self.camera_connected && self.raw_supported && self.focus_locked && self.storage_ok && self.thermal_ok
    }
}
