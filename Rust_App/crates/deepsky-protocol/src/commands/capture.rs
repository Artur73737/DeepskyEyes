//! CAPTURE: CAPTURE, CANCEL_CAPTURE.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CaptureCommand {
    Capture { request_id: u32 },
    CancelCapture { request_id: u32 },
}
