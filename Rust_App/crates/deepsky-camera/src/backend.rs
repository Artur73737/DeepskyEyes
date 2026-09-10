//! Backend astratto: PixelCameraBackend oggi, DSLR/astro/simulator domani (README §88).
use crate::model::*;
pub trait CameraBackend {
    fn id(&self) -> &str;
    fn discover(&mut self) -> CameraResult<Vec<CameraCapabilities>>;
    fn open(&mut self, selection: &CameraSelection) -> CameraResult<()>;
    fn configure(&mut self, request: &CaptureRequest, policy: ValidationPolicy) -> CameraResult<ConfigurationOutcome>;
    fn capture(&mut self) -> CameraResult<CapturedFrame>;
    fn preview(&mut self) -> CameraResult<PreviewFrame>;
    fn thermal(&mut self) -> CameraResult<ThermalStatus>;
    fn close(&mut self) -> CameraResult<()>;
}
