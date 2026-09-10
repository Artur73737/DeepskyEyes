//! Eventi asincroni Android -> PC (README §31).

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Event {
    DeviceConnected,
    DeviceDisconnected,
    CameraOpened { camera_id: String },
    CameraClosed { camera_id: String },
    CameraError { message: String },
    CaptureStarted { request_id: u32 },
    CaptureCompleted { request_id: u32 },
    CaptureFailed { request_id: u32, reason: String },
    FrameAvailable { frame_id: String },
    FrameTransferStarted { frame_id: String },
    FrameTransferCompleted { frame_id: String },
    TemperatureChanged { celsius_tenths: i32 },
    ThermalWarning { level: u8 },
    SequenceStarted { sequence_id: String },
    SequenceProgress { sequence_id: String, done: u32, total: u32 },
    SequencePaused { sequence_id: String },
    SequenceCompleted { sequence_id: String },
    SequenceFailed { sequence_id: String, reason: String },
}
