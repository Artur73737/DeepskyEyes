//! Validazione pre-start (README §36).
#[derive(Debug, Clone, Default)]
pub struct SequenceValidation {
    pub camera_connected: bool,
    pub camera_open: bool,
    pub raw_supported: bool,
    pub focus_locked: bool,
    pub storage_ok: bool,
    pub thermal_ok: bool,
    pub white_balance_fixed: bool,
    pub transport_ok: bool,
    pub clock_valid: bool,
    /// Free bytes when the platform reports them. None means unknown: the
    /// size check is skipped and the session create + write path proves
    /// writability instead of pretending a number.
    pub available_bytes: Option<u64>,
    /// Conservative estimate including RAW, metadata and overhead.
    pub bytes_per_frame: u64,
}

impl SequenceValidation {
    pub fn ok(&self) -> bool {
        self.camera_connected && self.camera_open && self.raw_supported && self.focus_locked
            && self.storage_ok && self.thermal_ok && self.white_balance_fixed
            && self.transport_ok && self.clock_valid
    }

    pub fn validate(&self, plan: &super::plan::SequencePlan, capabilities: &deepsky_camera::model::CameraCapabilities,
        request: &deepsky_camera::model::CaptureRequest) -> Result<(), super::errors::SequencerError> {
        use deepsky_camera::model::{FocusRequest, PixelFormat, WhiteBalanceRequest, ValidationPolicy};
        use super::errors::SequencerError::ValidationFailed;
        if !self.ok() { return Err(ValidationFailed("preflight status not safe")); }
        if plan.frames == 0 || plan.exposure_ns == 0 || plan.sensitivity == 0 { return Err(ValidationFailed("empty or zero plan")); }
        if plan.exposure_ns.checked_add(plan.delay_ns).and_then(|v| v.checked_mul(u64::from(plan.frames))).is_none() { return Err(ValidationFailed("plan duration overflow")); }
        let required = self.bytes_per_frame.checked_mul(u64::from(plan.frames)).ok_or(ValidationFailed("storage estimate overflow"))?;
        if self.bytes_per_frame == 0 { return Err(ValidationFailed("storage estimate overflow")); }
        if let Some(available) = self.available_bytes {
            if available < required { return Err(ValidationFailed("insufficient storage")); }
        }
        let s = &request.settings;
        if s.exposure_ns != Some(plan.exposure_ns) || s.sensitivity != Some(u64::from(plan.sensitivity)) { return Err(ValidationFailed("plan/request mismatch")); }
        if capabilities.raw != Some(true) || !s.stream.as_ref().is_some_and(|s| matches!(s.format, PixelFormat::Raw16Le | PixelFormat::Dng)) { return Err(ValidationFailed("RAW stream required")); }
        if !matches!(s.focus, Some(FocusRequest::Manual { locked: true, .. }) | Some(FocusRequest::Auto { locked: true, .. })) { return Err(ValidationFailed("focus must be locked")); }
        let fixed = match &s.white_balance {
            Some(WhiteBalanceRequest::Manual { .. } | WhiteBalanceRequest::Temperature { .. }) => true,
            Some(WhiteBalanceRequest::Mode(m)) => matches!(m.as_str(), "incandescent" | "fluorescent" | "warm_fluorescent" | "daylight" | "cloudy_daylight" | "twilight" | "shade"),
            None => false,
        };
        if !fixed { return Err(ValidationFailed("fixed white balance required")); }
        capabilities.validate_request(request, ValidationPolicy::Reject).map_err(|e| super::errors::SequencerError::Camera(e.to_string()))?;
        Ok(())
    }
}
