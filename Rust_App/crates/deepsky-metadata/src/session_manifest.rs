//! Manifest sessione machine-readable (README §41).
#[derive(Debug, Clone, Default)]
pub struct SessionManifest {
    pub project: String,
    pub device: String,
    pub camera_id: String,
    pub frames_requested: u32,
    pub frames_completed: u32,
}
