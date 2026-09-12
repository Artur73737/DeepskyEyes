//! Backend astratto: PixelCameraBackend oggi, DSLR/astro/simulator domani (README §88).
use crate::model::*;
pub trait CameraBackend {
    fn id(&self) -> &str;
    fn discover(&mut self) -> CameraResult<Vec<CameraCapabilities>>;
    fn open(&mut self, selection: &CameraSelection) -> CameraResult<()>;
    fn configure(&mut self, request: &CaptureRequest, policy: ValidationPolicy) -> CameraResult<ConfigurationOutcome>;
    fn capture(&mut self) -> CameraResult<CapturedFrame>;
    fn preview(&mut self) -> CameraResult<PreviewFrame>;
    /// Center AF, returning the measured locked distance in millidiopters.
    fn autofocus_center(&mut self) -> CameraResult<u64> {
        Err(CameraError::new(ErrorCode::Unsupported, "Center autofocus unavailable on this backend"))
    }
    fn thermal(&mut self) -> CameraResult<ThermalStatus>;
    /// Read-only adapter diagnostics, separate from the portable capability model.
    fn diagnostic(&mut self, _method: &str) -> CameraResult<serde_json::Value> {
        Err(CameraError::new(ErrorCode::Unsupported, "adapter diagnostics unavailable"))
    }
    fn close(&mut self) -> CameraResult<()>;
    /// Cumulative (rx, tx) transport bytes when counted; None for direct links.
    fn transport_stats(&self) -> Option<(u64, u64)> {
        None
    }
}
