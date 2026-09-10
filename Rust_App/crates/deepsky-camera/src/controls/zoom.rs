//! Zoom ottico/digitale + crop region (SCALER_CROP_REGION).
//!
//! Zoom Pixel 8 Pro (ricerca doc/01): wide 2x da crop sensore, tele 5x ottico,
//! Super Res fino a 30x. I range reali vanno letti da
//! `CONTROL_ZOOM_RATIO_RANGE` + `SCALER_AVAILABLE_MAX_DIGITAL_ZOOM`.

/// Regione di crop in pixel del sensore (SCALER_CROP_REGION).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CropRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Controllo zoom con requested/applied/reported (×1000: 1000=1.0x, 5000=5.0x).
#[derive(Debug, Clone, Copy, Default)]
pub struct ZoomControl {
    pub requested_x1000: u32,
    pub applied_x1000: Option<u32>,
    pub reported_x1000: Option<u32>,
    /// Zoom digitale massimo annunciato dal device (0 = sconosciuto).
    pub max_digital_zoom_x1000: u32,
    pub crop: Option<CropRegion>,
}

impl ZoomControl {
    pub fn new_1x() -> Self {
        Self { requested_x1000: 1000, applied_x1000: None, reported_x1000: None, max_digital_zoom_x1000: 0, crop: None }
    }

    /// true only inside the announced range. Unknown maximum means only 1.0x
    /// is allowed — zoom support is never assumed.
    pub fn is_supported(&self, ratio_x1000: u32) -> bool {
        if self.max_digital_zoom_x1000 == 0 {
            return false;
        }
        (1000..=self.max_digital_zoom_x1000).contains(&ratio_x1000)
    }
}
