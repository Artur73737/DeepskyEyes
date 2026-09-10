//! Stato applicativo consumato dalla UI (mai logica USB/camera qui).
#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub connected: bool,
    pub camera_id: Option<String>,
    pub frames_done: u32,
    pub frames_total: u32,
}
