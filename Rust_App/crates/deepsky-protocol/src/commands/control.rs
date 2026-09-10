//! CONTROL: exposure, sensitivity, frame, focus, AF, AWB, gains, zoom, crop, processing.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum ControlCommand {
    SetExposure { exposure_ns: u64 },
    SetSensitivity { iso: u32 },
    SetFrameDuration { frame_duration_ns: u64 },
    SetFocus { focus_distance_diopters: f32 },
    SetAfMode { mode: AfMode },
    SetAwb { mode: AwbMode },
    SetColorGains { r: f32, g_even: f32, g_odd: f32, b: f32 },
    SetZoom { ratio_x1000: u32 },
    SetCrop { x: u32, y: u32, w: u32, h: u32 },
    SetResolution { width: u32, height: u32, format: u8 },
    SetProcessingMode { edge: u8, noise: u8, hot_pixel: u8, shading: u8, tonemap: u8 },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfMode {
    Off,
    Auto,
    ContinuousPicture,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwbMode {
    Off,
    Auto,
    Daylight,
    Cloudy,
}
