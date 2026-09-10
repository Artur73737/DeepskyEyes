//! WB: AUTO/PRESET/TEMPERATURE/TINT/MANUAL_GAINS (README §18).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhiteBalance {
    Auto,
    Preset(u8),
    Temperature { kelvin: u16 },
    ManualGains { r: f32, g_even: f32, g_odd: f32, b: f32 },
}
