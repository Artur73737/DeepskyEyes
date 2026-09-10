//! DEVICE: GET_DEVICE_INFO, GET_CAMERAS, GET_CAMERA_CAPABILITIES.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DeviceCommand {
    GetDeviceInfo,
    GetCameras,
    GetCameraCapabilities { camera_id: String },
}
