//! Capability grezze dal device (dump CameraCharacteristics).
#[derive(Debug, Clone, Default)]
pub struct RawCapabilities {
    pub camera_id: String,
    pub lens_facing: String,
    pub hardware_level: String,
    pub raw_supported: bool,
    pub manual_sensor: bool,
    pub manual_post_processing: bool,
    pub logical_multi_camera: bool,
    /// Size (w, h) annunciate dallo StreamConfigurationMap per il formato RAW.
    pub raw_sizes: Vec<(u32, u32)>,
    /// Zoom digitale massimo annunciato (×1000, 0 = sconosciuto).
    pub max_digital_zoom_x1000: u32,
}
