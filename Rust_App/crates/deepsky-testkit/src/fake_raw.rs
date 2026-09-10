//! Generated Bayer stars; not a hardware calibration or DNG.
use deepsky_camera::*;
pub fn star_raw16(width: u32, height: u32, seed: u64, frame: u64, settings: &CaptureSettings) -> CameraResult<Vec<u16>> {
    let count = u64::from(width) * u64::from(height);
    if count == 0 || count > 16_777_216 { return Err(CameraError::new(ErrorCode::InvalidRequest, "synthetic image allocation bound")); }
    let mut state = seed ^ frame.wrapping_mul(0x9e3779b97f4a7c15);
    let scale = settings.exposure_ns.unwrap_or(1_000_000) as f64 / 1_000_000.0 * settings.sensitivity.unwrap_or(100) as f64 / 100.0;
    let focus = match settings.focus { Some(FocusRequest::Manual { millidiopters, .. }) => millidiopters as f64 / 1000.0, _ => 0.0 }; let sigma = 1.0 + focus;
    let zoom = settings.zoom_x1000.unwrap_or(1000) as f64 / 1000.0; let mut pixels = Vec::with_capacity(count as usize);
    for y in 0..height { for x in 0..width {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); let mut signal = 0.0;
        for (sx, sy, amplitude) in [(0.25,0.3,24000.0),(0.65,0.55,42000.0),(0.45,0.8,16000.0)] {
            let dx = x as f64 - ((sx - 0.5) * zoom + 0.5) * width as f64; let dy = y as f64 - ((sy - 0.5) * zoom + 0.5) * height as f64;
            signal += amplitude / (sigma*sigma) * (-(dx*dx+dy*dy)/(2.0*sigma*sigma)).exp();
        } pixels.push((256.0 + (state >> 60) as f64 + signal * scale).min(65535.0) as u16);
    } } Ok(pixels)
}
pub fn fake_raw_bytes(width: u32, height: u32) -> Vec<u16> { star_raw16(width, height, 42, 0, &CaptureSettings::default()).unwrap_or_default() }
