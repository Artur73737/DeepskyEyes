//! Controllo zoom (×1000: 1000=1.0x, 2000=2.0x, 5000=5.0x) + crop.
//! Range reale da CONTROL_ZOOM_RATIO_RANGE / SCALER_AVAILABLE_MAX_DIGITAL_ZOOM.
pub struct ZoomControls {
    pub ratio_x1000: u32,
    /// Zoom massimo annunciato (0 = sconosciuto, nessun clamp applicato).
    pub max_x1000: u32,
}
