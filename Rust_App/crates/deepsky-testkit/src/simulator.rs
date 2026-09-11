//! Synthetic instrument; these capabilities are not Pixel measurements.
use deepsky_camera::{backend::CameraBackend, *};
use serde::{Deserialize, Serialize};
use crate::failure_injection::InjectedFailure;
pub const SYNTHETIC_CAMERA_ID: &str = "SYNTHETIC-STAR-CAMERA";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimingMode { #[default] Accelerated, Realtime }
pub struct SimulatorBackend { timing: TimingMode, seed: u64, opened: bool, configuration: Option<ConfigurationOutcome>, frame: u64, clock_ns: u64, failure: Option<InjectedFailure>, thermal_warning: bool }
impl Default for SimulatorBackend { fn default() -> Self { Self::new(TimingMode::Accelerated) } }
impl SimulatorBackend {
    pub fn new(timing: TimingMode) -> Self { Self::with_seed(timing, 42) }
    pub fn with_seed(timing: TimingMode, seed: u64) -> Self { Self { timing, seed, opened: false, configuration: None, frame: 0, clock_ns: 0, failure: None, thermal_warning: false } }
    pub fn inject_failure(&mut self, failure: InjectedFailure) { self.failure = Some(failure); }
    pub fn elapsed_ns(&self) -> u64 { self.clock_ns }
    pub fn capabilities() -> CameraCapabilities {
        CameraCapabilities {
            schema_version: 1, camera_id: SYNTHETIC_CAMERA_ID.into(), identity: "SYNTHETIC deterministic star generator v1".into(), origin: Some(Origin::Synthetic), logical: Some(false), manual_sensor: Some(true), raw: Some(true),
            exposure_ns: Some(ValueRange { min: 100_000, max: 30_000_000_000 }), sensitivity: Some(ValueRange { min: 100, max: 6400 }), frame_duration_ns: Some(ValueRange { min: 1_000_000, max: 31_000_000_000 }),
            focus_millidiopters: Some(ValueRange { min: 0, max: 10000 }), focus_calibration: Some("synthetic calibrated".into()), af_modes: vec!["off".into()], focus_lock: Some(true), zoom_x1000: Some(ValueRange { min: 1000, max: 4000 }),
            streams: vec![StreamConfiguration { width: 128, height: 96, format: PixelFormat::Raw16Le, pixel_mode: SensorPixelMode::Default, binned: Some(false), min_frame_duration_ns: Some(1_000_000), stall_duration_ns: Some(500_000) }],
            preview_streams: vec![StreamConfiguration { width: 128, height: 96, format: PixelFormat::Gray8, pixel_mode: SensorPixelMode::Default, binned: Some(false), min_frame_duration_ns: Some(33_333_333), stall_duration_ns: Some(0) }],
            wb_modes: vec!["manual".into()], wb_gain_x1000: Some(ValueRange { min: 1000, max: 8000 }),
            processing_modes: ["edge", "noise_reduction", "hot_pixel", "shading", "tonemap", "aberration", "distortion"].into_iter().map(|k| (k.into(), vec!["off".into()])).collect(), ois_modes: vec!["off".into()], eis_modes: vec!["off".into()], ..Default::default()
        }
    }
    pub fn default_request() -> CaptureRequest { CaptureRequest { request_id: 1, selection: CameraSelection { camera_id: SYNTHETIC_CAMERA_ID.into(), physical_id: None }, settings: CaptureSettings { exposure_ns: Some(1_000_000), sensitivity: Some(100), frame_duration_ns: Some(1_000_000), focus: Some(FocusRequest::Manual { millidiopters: 0, locked: true }), stream: Some(Self::capabilities().streams.remove(0)), ..Default::default() } } }
    fn check(&mut self) -> CameraResult<()> {
        if let Some(failure) = self.failure.take() { let code = match failure {
            InjectedFailure::UsbDisconnect | InjectedFailure::AndroidCrash => { self.opened = false; self.configuration = None; ErrorCode::Disconnected },
            InjectedFailure::DiskFull => ErrorCode::Io, InjectedFailure::TransportTimeout => ErrorCode::Timeout,
            InjectedFailure::ThermalWarning => { self.thermal_warning = true; return Ok(()); },
        }; return Err(CameraError::new(code, format!("SYNTHETIC injected {failure:?}"))); } Ok(())
    }
    fn ready(&self) -> CameraResult<&ConfigurationOutcome> { self.configuration.as_ref().filter(|_| self.opened).ok_or_else(|| CameraError::new(ErrorCode::InvalidState, "configure an open camera first")) }
}
impl CameraBackend for SimulatorBackend {
    fn id(&self) -> &str { SYNTHETIC_CAMERA_ID }
    fn discover(&mut self) -> CameraResult<Vec<CameraCapabilities>> { self.check()?; Ok(vec![Self::capabilities()]) }
    fn open(&mut self, selection: &CameraSelection) -> CameraResult<()> { self.check()?; if self.opened { return Err(CameraError::new(ErrorCode::InvalidState, "already open")); } if selection.camera_id != SYNTHETIC_CAMERA_ID || selection.physical_id.is_some() { return Err(CameraError::new(ErrorCode::Unsupported, "synthetic camera selection")); } self.opened = true; Ok(()) }
    fn configure(&mut self, request: &CaptureRequest, policy: ValidationPolicy) -> CameraResult<ConfigurationOutcome> {
        self.check()?; if !self.opened { return Err(CameraError::new(ErrorCode::InvalidState, "camera closed")); }
        let outcome = Self::capabilities().validate_request(request, policy)?; let s = &outcome.applied.settings;
        if s.exposure_ns.is_none() || s.sensitivity.is_none() || s.stream.is_none() || s.focus.is_none() { return Err(CameraError::new(ErrorCode::InvalidRequest, "explicit exposure, sensitivity, stream and focus required")); }
        self.configuration = Some(outcome.clone()); Ok(outcome)
    }
    fn capture(&mut self) -> CameraResult<CapturedFrame> {
        self.check()?; let configuration = self.ready()?.clone(); let settings = &configuration.applied.settings; let stream = settings.stream.as_ref().unwrap(); let width = stream.width;
        let period = settings.frame_duration_ns.unwrap_or(settings.exposure_ns.unwrap().max(stream.min_frame_duration_ns.unwrap_or(0)));
        let duration = period.checked_add(stream.stall_duration_ns.unwrap_or(0)).ok_or_else(|| CameraError::new(ErrorCode::InvalidRequest, "timing overflow"))?;
        let end = self.clock_ns.checked_add(duration).ok_or_else(|| CameraError::new(ErrorCode::InvalidState, "clock exhausted"))?;
        if self.timing == TimingMode::Realtime { std::thread::sleep(std::time::Duration::from_nanos(duration)); }
        let pixels = crate::fake_raw::star_raw16(width, stream.height, self.seed, self.frame, settings)?;
        let payload = pixels.into_iter().flat_map(u16::to_le_bytes).collect(); let mut reported = settings.clone(); reported.frame_duration_ns = Some(period);
        let metadata = CaptureMetadata { identity: Self::capabilities().identity, origin: Origin::Synthetic, frame_id: self.frame, configuration, reported, active_physical_id: None, timestamp_domain: "SYNTHETIC monotonic nanoseconds".into(), start_ns: self.clock_ns, end_ns: end, raw_layout: Some(RawLayout { row_stride_bytes: width * 2, pixel_stride_bytes: 2, bit_depth: 16, cfa: Some("RGGB".into()), black_levels: Some([256;4]), white_level: Some(65535) }) };
        self.clock_ns = end; self.frame += 1; Ok(CapturedFrame { payload, metadata })
    }
    fn preview(&mut self) -> CameraResult<PreviewFrame> { self.check()?; let config = self.ready()?; let pixels = crate::fake_raw::star_raw16(128, 96, self.seed, self.frame, &config.applied.settings)?; Ok(PreviewFrame { payload: pixels.into_iter().map(|v| (v >> 8) as u8).collect(), width: 128, height: 96, format: PixelFormat::Gray8, timestamp_ns: self.clock_ns, origin: Origin::Synthetic, reported_exposure_ns: config.applied.settings.exposure_ns, reported_sensitivity: config.applied.settings.sensitivity }) }
    fn thermal(&mut self) -> CameraResult<ThermalStatus> { self.check()?; Ok(ThermalStatus { severity: if self.thermal_warning { "warning" } else { "nominal" }.into(), temperature_c: Some(if self.thermal_warning { 48.0 } else { 25.0 }), origin: Origin::Synthetic }) }
    fn close(&mut self) -> CameraResult<()> { self.check()?; self.opened = false; self.configuration = None; Ok(()) }
}
