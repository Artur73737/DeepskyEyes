//! CAMERA: OPEN, CLOSE, CONFIGURE, GET_STATE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraCommand {
    OpenCamera { camera_id: String },
    CloseCamera { camera_id: String },
    ConfigureCamera { camera_id: String },
    GetCameraState { camera_id: String },
}
