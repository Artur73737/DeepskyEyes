//! Modello capability validato -> UI availability (README §34).
use crate::capabilities::RawCapabilities;

#[derive(Debug, Clone, Default)]
pub struct CapabilityModel {
    pub raw: bool,
    pub manual_exposure: bool,
    pub manual_focus: bool,
    pub manual_white_balance: bool,
}

impl CapabilityModel {
    pub fn from_raw(raw: &RawCapabilities) -> Self {
        Self {
            raw: raw.raw_supported,
            manual_exposure: raw.manual_sensor,
            manual_focus: raw.manual_sensor,
            manual_white_balance: raw.manual_post_processing,
        }
    }
}
