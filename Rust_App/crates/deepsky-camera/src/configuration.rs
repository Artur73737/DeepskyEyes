//! Configurazione camera con Requested/Applied/Reported (README §106).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState<T> {
    Requested(T),
    Applied(T),
    Reported(T),
}

#[derive(Debug, Clone, Default)]
pub struct CameraConfiguration {
    pub camera_id: String,
    pub exposure_ns_requested: Option<u64>,
    pub exposure_ns_applied: Option<u64>,
    pub sensitivity_requested: Option<u32>,
    pub sensitivity_applied: Option<u32>,
    pub focus_locked: bool,
}
