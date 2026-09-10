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
}
