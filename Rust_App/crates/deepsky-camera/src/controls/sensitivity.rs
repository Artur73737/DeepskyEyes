//! Sensibilità sensore (non ISO marketing).
#[derive(Debug, Clone, Copy)]
pub struct SensitivityControl {
    pub requested: u32,
    pub applied: Option<u32>,
    pub reported: Option<u32>,
}
