//! Modello capability validato -> UI availability (README §34).
use crate::capabilities::RawCapabilities;

#[derive(Debug, Clone, Default)]
pub struct CapabilityModel {
    pub raw: bool,
    pub manual_exposure: bool,
    pub manual_focus: bool,
    pub manual_white_balance: bool,
    pub zoom_supported: bool,
    pub full_resolution_supported: bool,
}

impl CapabilityModel {
    pub fn from_raw(raw: &RawCapabilities) -> Self {
        Self {
            raw: raw.raw_supported,
            manual_exposure: raw.manual_sensor,
            manual_focus: raw.manual_sensor,
            manual_white_balance: raw.manual_post_processing,
            zoom_supported: raw.max_digital_zoom_x1000 > 1000,
            full_resolution_supported: raw.raw_sizes.len() > 1,
        }
    }
}
