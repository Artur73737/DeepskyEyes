//! CAMERA: OPEN, CLOSE, CONFIGURE, GET_STATE, SELECT_LENS.
use crate::wire_types::LensType;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CameraCommand {
    OpenCamera { camera_id: String },
    CloseCamera { camera_id: String },
    ConfigureCamera { camera_id: String },
    GetCameraState { camera_id: String },
    SelectLens { lens: LensType },
}
