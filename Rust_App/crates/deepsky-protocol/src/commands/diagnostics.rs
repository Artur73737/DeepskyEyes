//! DIAGNOSTICS: GET_LOG, GET_METRICS, GET_THERMAL_STATUS, GET_CAMERA_RESULT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsCommand {
    GetLog { lines: u32 },
    GetMetrics,
    GetThermalStatus,
    GetCameraResult { request_id: u32 },
}
