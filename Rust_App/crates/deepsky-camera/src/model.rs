//! Portable camera contract. Missing evidence stays `None`/empty, never supported by default.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraError { pub code: ErrorCode, pub message: String }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode { Unsupported, OutOfRange, InvalidCapabilities, InvalidState, InvalidRequest, Disconnected, Timeout, Thermal, Io }
impl CameraError { pub fn new(code: ErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into() } } }
impl std::fmt::Display for CameraError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}: {}", self.code, self.message) } }
impl std::error::Error for CameraError {}
pub type CameraResult<T> = Result<T, CameraError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueRange { pub min: u64, pub max: u64 }
impl ValueRange {
    pub fn validate(&self) -> CameraResult<()> { if self.min > self.max { Err(CameraError::new(ErrorCode::InvalidCapabilities, "inverted range")) } else { Ok(()) } }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin { Synthetic, Device }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat { Raw16Le, Gray8, Rgb8, Jpeg, Dng, Yuv420, Private }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorPixelMode { Default, MaximumResolution }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamConfiguration {
    pub width: u32, pub height: u32, pub format: PixelFormat,
    pub pixel_mode: SensorPixelMode, pub binned: Option<bool>,
    pub min_frame_duration_ns: Option<u64>, pub stall_duration_ns: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraSelection { pub camera_id: String, pub physical_id: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalCamera { pub id: String, pub directly_openable: Option<bool>, pub routable: bool }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraCapabilities {
    pub schema_version: u32, pub camera_id: String, pub identity: String, pub origin: Option<Origin>,
    pub lens_facing: Option<String>, pub lens_role: Option<String>, pub hardware_level: Option<String>,
    pub logical: Option<bool>, pub physical_cameras: Vec<PhysicalCamera>,
    pub manual_sensor: Option<bool>, pub raw: Option<bool>,
    pub exposure_ns: Option<ValueRange>, pub sensitivity: Option<ValueRange>, pub frame_duration_ns: Option<ValueRange>,
    /// Fixed point diopters, 1000 = 1 diopter. None means unknown, not fixed focus.
    pub focus_millidiopters: Option<ValueRange>, pub focus_calibration: Option<String>,
    pub af_modes: Vec<String>, pub focus_lock: Option<bool>,
    #[serde(default)] pub autofocus_center_supported: Option<bool>,
    pub zoom_x1000: Option<ValueRange>, pub active_array: Option<CropRect>,
    pub crop_supported: Option<bool>, pub streams: Vec<StreamConfiguration>, pub preview_streams: Vec<StreamConfiguration>,
    pub wb_modes: Vec<String>, pub wb_kelvin: Option<ValueRange>, pub wb_tint: Option<ValueRange>,
    pub wb_gain_x1000: Option<ValueRange>,
    pub processing_modes: BTreeMap<String, Vec<String>>,
    pub ois_modes: Vec<String>, pub eis_modes: Vec<String>,
    /// Thumbnail sizes from `JPEG_AVAILABLE_THUMBNAIL_SIZES`, in device order.
    /// Empty means unknown (old dumps), not "no thumbnails".
    #[serde(default)] pub jpeg_thumbnail_sizes: Vec<JpegSize>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JpegSize { pub width: u32, pub height: u32 }
/// Typed JPEG output controls (Camera2 `android.jpeg.*`). Every field is optional:
/// absent means "HAL default", never an inferred value. Only meaningful with a
/// JPEG stream; setting any of them on RAW/other streams is rejected outright.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JpegSettings {
    #[serde(default)] pub quality: Option<u8>,
    #[serde(default)] pub orientation: Option<u16>,
    #[serde(default)] pub thumbnail_quality: Option<u8>,
    #[serde(default)] pub thumbnail_size: Option<JpegSize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusRequest { Manual { millidiopters: u64, locked: bool }, Auto { mode: String, locked: bool } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhiteBalanceRequest { Mode(String), Temperature { kelvin: u64, tint: Option<u64> }, Manual { gains_x1000: [u64; 4], transform_millionths: [i32; 9] } }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureSettings {
    pub exposure_ns: Option<u64>, pub sensitivity: Option<u64>, pub frame_duration_ns: Option<u64>,
    pub focus: Option<FocusRequest>, pub zoom_x1000: Option<u64>, pub crop: Option<CropRect>,
    pub stream: Option<StreamConfiguration>, pub white_balance: Option<WhiteBalanceRequest>,
    pub processing: BTreeMap<String, String>, pub ois: Option<String>, pub eis: Option<String>,
    #[serde(default)] pub jpeg: Option<JpegSettings>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRequest { pub request_id: u64, pub selection: CameraSelection, pub settings: CaptureSettings }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationPolicy { Reject, Clamp }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjustment { pub field: String, pub requested: u64, pub applied: u64 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationOutcome { pub requested: CaptureRequest, pub applied: CaptureRequest, pub adjustments: Vec<Adjustment> }

fn numeric(field: &str, value: &mut Option<u64>, range: Option<ValueRange>, policy: ValidationPolicy, changes: &mut Vec<Adjustment>) -> CameraResult<()> {
    if let Some(v) = *value {
        let r = range.ok_or_else(|| CameraError::new(ErrorCode::Unsupported, field))?;
        r.validate()?;
        let applied = v.clamp(r.min, r.max);
        if applied != v {
            if policy == ValidationPolicy::Reject { return Err(CameraError::new(ErrorCode::OutOfRange, field)); }
            changes.push(Adjustment { field: field.into(), requested: v, applied }); *value = Some(applied);
        }
    } Ok(())
}
fn mode(field: &str, value: &str, supported: &[String]) -> CameraResult<()> {
    if supported.iter().any(|x| x == value) { Ok(()) } else { Err(CameraError::new(ErrorCode::Unsupported, field)) }
}
impl CameraCapabilities {
    /// Validate a routed request using the physical sensor's own discovery snapshot.
    pub fn validate_physical_request(&self, physical: &Self, request: &CaptureRequest, policy: ValidationPolicy) -> CameraResult<ConfigurationOutcome> {
        if request.selection.camera_id != self.camera_id || self.logical != Some(true) || request.selection.physical_id.as_deref() != Some(physical.camera_id.as_str()) || !self.physical_cameras.iter().any(|p| p.id == physical.camera_id && p.routable) { return Err(CameraError::new(ErrorCode::Unsupported, "physical routing")); }
        let mut direct = request.clone(); direct.selection = CameraSelection { camera_id: physical.camera_id.clone(), physical_id: None };
        let mut outcome = physical.validate_request(&direct, policy)?;
        outcome.requested = request.clone(); outcome.applied.selection = request.selection.clone(); Ok(outcome)
    }
    pub fn validate(&self) -> CameraResult<()> {
        for r in [self.exposure_ns, self.sensitivity, self.frame_duration_ns, self.focus_millidiopters, self.zoom_x1000, self.wb_kelvin, self.wb_tint, self.wb_gain_x1000].into_iter().flatten() { r.validate()?; }
        // frame_duration_ns is exempt: a zero minimum means "no lower bound
        // announced" (SENSOR_INFO_MAX_FRAME_DURATION only carries a maximum,
        // as seen on Pixel 8 Pro), not a broken capability.
        for r in [self.exposure_ns, self.sensitivity, self.zoom_x1000, self.wb_kelvin, self.wb_gain_x1000].into_iter().flatten() { if r.min == 0 { return Err(CameraError::new(ErrorCode::InvalidCapabilities, "positive range has zero minimum")); } }
        if self.streams.iter().chain(&self.preview_streams).any(|s| s.width == 0 || s.height == 0) { return Err(CameraError::new(ErrorCode::InvalidCapabilities, "zero stream size")); }
        Ok(())
    }
    pub fn validate_request(&self, request: &CaptureRequest, policy: ValidationPolicy) -> CameraResult<ConfigurationOutcome> {
        self.validate()?;
        if request.selection.camera_id != self.camera_id { return Err(CameraError::new(ErrorCode::Unsupported, "camera ID")); }
        if let Some(id) = &request.selection.physical_id {
            if self.logical != Some(true) || !self.physical_cameras.iter().any(|p| &p.id == id && p.routable) { return Err(CameraError::new(ErrorCode::Unsupported, "physical routing")); }
            // Physical controls require their own discovered capability snapshot.
            return Err(CameraError::new(ErrorCode::Unsupported, "validate against physical camera capabilities before routing"));
        }
        let mut applied = request.clone(); let s = &mut applied.settings; let mut adjustments = vec![];
        if (s.exposure_ns.is_some() || s.sensitivity.is_some() || s.frame_duration_ns.is_some()) && self.manual_sensor != Some(true) { return Err(CameraError::new(ErrorCode::Unsupported, "manual sensor / AE OFF")); }
        for (name, value, range) in [("exposure_ns", &mut s.exposure_ns, self.exposure_ns), ("sensitivity", &mut s.sensitivity, self.sensitivity), ("frame_duration_ns", &mut s.frame_duration_ns, self.frame_duration_ns), ("zoom_x1000", &mut s.zoom_x1000, self.zoom_x1000)] { numeric(name, value, range, policy, &mut adjustments)?; }
        if let Some(stream) = &s.stream {
            if stream.width == 0 || stream.height == 0 || !self.streams.contains(stream) { return Err(CameraError::new(ErrorCode::Unsupported, "stream size/format/pixel mode")); }
            if stream.format == PixelFormat::Raw16Le && self.raw != Some(true) { return Err(CameraError::new(ErrorCode::Unsupported, "RAW")); }
        }
        let minimum = s.exposure_ns.unwrap_or(0).max(s.stream.as_ref().and_then(|s| s.min_frame_duration_ns).unwrap_or(0));
        if self.frame_duration_ns.is_some_and(|r| minimum > r.max) { return Err(CameraError::new(ErrorCode::InvalidRequest, "exposure/stream minimum exceeds maximum frame duration")); }
        if s.frame_duration_ns.is_some_and(|v| v < minimum) { return Err(CameraError::new(ErrorCode::InvalidRequest, "frame duration shorter than exposure/stream minimum")); }
        if let Some(focus) = &mut s.focus {
            match focus {
                FocusRequest::Manual { millidiopters, locked } => {
                    mode("AF OFF", "off", &self.af_modes)?;
                    let mut value = Some(*millidiopters); numeric("focus_millidiopters", &mut value, self.focus_millidiopters, policy, &mut adjustments)?; *millidiopters = value.unwrap();
                    if *locked && self.focus_lock != Some(true) { return Err(CameraError::new(ErrorCode::Unsupported, "focus lock")); }
                }
                FocusRequest::Auto { mode: m, locked } => { mode("AF", m, &self.af_modes)?; if m == "off" || (*locked && self.focus_lock != Some(true)) { return Err(CameraError::new(ErrorCode::Unsupported, "AF lock/mode")); } }
            }
        }
        if let Some(crop) = s.crop {
            let a = self.active_array.ok_or_else(|| CameraError::new(ErrorCode::Unsupported, "crop active array"))?;
            if self.crop_supported != Some(true) || crop.width == 0 || crop.height == 0 || crop.x < a.x || crop.y < a.y || u64::from(crop.x)+u64::from(crop.width) > u64::from(a.x)+u64::from(a.width) || u64::from(crop.y)+u64::from(crop.height) > u64::from(a.y)+u64::from(a.height) { return Err(CameraError::new(ErrorCode::OutOfRange, "crop")); }
        }
        if let Some(wb) = &mut s.white_balance {
            match wb {
                WhiteBalanceRequest::Mode(m) => { if m == "manual" || m == "temperature" { return Err(CameraError::new(ErrorCode::InvalidRequest, "WB mode requires its parameterized request")); } mode("WB", m, &self.wb_modes)?; },
                WhiteBalanceRequest::Temperature { kelvin, tint } => {
                    mode("WB temperature", "temperature", &self.wb_modes)?;
                    let mut v = Some(*kelvin); numeric("wb_kelvin", &mut v, self.wb_kelvin, policy, &mut adjustments)?; *kelvin = v.unwrap(); numeric("wb_tint", tint, self.wb_tint, policy, &mut adjustments)?;
                }
                WhiteBalanceRequest::Manual { gains_x1000, .. } => {
                    mode("WB manual", "manual", &self.wb_modes)?;
                    for (i, gain) in gains_x1000.iter_mut().enumerate() { let mut v = Some(*gain); numeric(&format!("wb_gain_{i}"), &mut v, self.wb_gain_x1000, policy, &mut adjustments)?; *gain = v.unwrap(); }
                }
            }
        }
        for (key, value) in &s.processing { mode(key, value, self.processing_modes.get(key).map(Vec::as_slice).unwrap_or(&[]))?; }
        if let Some(v) = &s.ois { mode("OIS", v, &self.ois_modes)?; } if let Some(v) = &s.eis { mode("EIS", v, &self.eis_modes)?; }
        if let Some(jpeg) = &s.jpeg {
            // JPEG controls are output encoding, not sensor state: on a non-JPEG
            // stream they would be silently ignored by the HAL, so refuse them.
            let active = jpeg.quality.is_some() || jpeg.orientation.is_some()
                || jpeg.thumbnail_quality.is_some() || jpeg.thumbnail_size.is_some();
            if active && !s.stream.as_ref().is_some_and(|st| st.format == PixelFormat::Jpeg) {
                return Err(CameraError::new(ErrorCode::InvalidRequest, "JPEG controls require a JPEG stream"));
            }
            // JPEG_QUALITY / JPEG_THUMBNAIL_QUALITY are documented as 1..100.
            if jpeg.quality.is_some_and(|q| q == 0 || q > 100) {
                return Err(CameraError::new(ErrorCode::OutOfRange, "jpeg_quality"));
            }
            if jpeg.thumbnail_quality.is_some_and(|q| q == 0 || q > 100) {
                return Err(CameraError::new(ErrorCode::OutOfRange, "jpeg_thumbnail_quality"));
            }
            // JPEG_ORIENTATION accepts exactly the four EXIF orientations.
            if jpeg.orientation.is_some_and(|o| ![0, 90, 180, 270].contains(&o)) {
                return Err(CameraError::new(ErrorCode::InvalidRequest, "jpeg_orientation"));
            }
            if let Some(size) = &jpeg.thumbnail_size {
                if !self.jpeg_thumbnail_sizes.contains(size) {
                    return Err(CameraError::new(ErrorCode::Unsupported, "jpeg thumbnail size not announced"));
                }
            }
        }
        Ok(ConfigurationOutcome { requested: request.clone(), applied, adjustments })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalStatus { pub severity: String, pub temperature_c: Option<f32>, pub origin: Origin }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawLayout { pub row_stride_bytes: u32, pub pixel_stride_bytes: u32, pub bit_depth: u8, pub cfa: Option<String>, pub black_levels: Option<[u16; 4]>, pub white_level: Option<u16> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureMetadata {
    pub identity: String, pub origin: Origin, pub frame_id: u64,
    pub configuration: ConfigurationOutcome,
    /// Only observations from this capture. Unknown fields remain absent.
    pub reported: CaptureSettings, pub active_physical_id: Option<String>,
    pub timestamp_domain: String, pub start_ns: u64, pub end_ns: u64,
    pub raw_layout: Option<RawLayout>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedFrame { pub payload: Vec<u8>, pub metadata: CaptureMetadata }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewFrame {
    pub payload: Vec<u8>, pub width: u32, pub height: u32, pub format: PixelFormat,
    pub timestamp_ns: u64, pub origin: Origin,
    #[serde(default)] pub reported_exposure_ns: Option<u64>,
    #[serde(default)] pub reported_sensitivity: Option<u64>,
}
