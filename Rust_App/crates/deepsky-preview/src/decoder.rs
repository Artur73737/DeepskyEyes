//! Real preview decode to packed RGB8 on CPU.
//!
//! Supported inputs (from `CameraCapabilities.preview_streams` evidence):
//! Gray8 passthrough, Rgb8 validation, YUV420 (BT.601 integer) conversion,
//! Raw16Le high-byte grayscale (preview aid only — NOT a debayer).
//! Anything else is rejected: the caller picks another announced stream.
use deepsky_camera::model::{PixelFormat, PreviewFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    EmptyFrame,
    SizeMismatch { expected: usize, actual: usize },
    UnsupportedFormat(PixelFormat),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DecodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedRgb {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

impl DecodedRgb {
    pub fn mean_luma(&self) -> f64 {
        if self.rgb.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.rgb.chunks_exact(3).map(|p| u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2])).sum();
        sum as f64 / (self.rgb.len() / 3) as f64 / 3.0
    }
}

pub fn preview_to_rgb8(frame: &PreviewFrame) -> Result<DecodedRgb, DecodeError> {
    let pixels = frame.width as usize * frame.height as usize;
    if frame.width == 0 || frame.height == 0 || pixels > 16_777_216 {
        return Err(DecodeError::EmptyFrame);
    }
    let expect = |n: usize| {
        if frame.payload.len() == n {
            Ok(())
        } else {
            Err(DecodeError::SizeMismatch { expected: n, actual: frame.payload.len() })
        }
    };
    match &frame.format {
        PixelFormat::Gray8 => {
            expect(pixels)?;
            let mut rgb = Vec::with_capacity(pixels * 3);
            for &v in &frame.payload {
                rgb.extend_from_slice(&[v, v, v]);
            }
            Ok(DecodedRgb { width: frame.width, height: frame.height, rgb })
        }
        PixelFormat::Rgb8 => {
            expect(pixels * 3)?;
            Ok(DecodedRgb { width: frame.width, height: frame.height, rgb: frame.payload.clone() })
        }
        PixelFormat::Yuv420 => {
            if frame.width % 2 != 0 || frame.height % 2 != 0 {
                return Err(DecodeError::SizeMismatch { expected: 0, actual: frame.payload.len() });
            }
            expect(pixels * 3 / 2)?;
            Ok(DecodedRgb { width: frame.width, height: frame.height, rgb: yuv420_to_rgb8(&frame.payload, frame.width, frame.height) })
        }
        PixelFormat::Raw16Le => {
            // Preview aid: sensor high byte as grayscale. Scientific use must
            // read the .raw16 payload with its sidecar metadata instead.
            expect(pixels * 2)?;
            let mut rgb = Vec::with_capacity(pixels * 3);
            for pair in frame.payload.chunks_exact(2) {
                let v = pair[1];
                rgb.extend_from_slice(&[v, v, v]);
            }
            Ok(DecodedRgb { width: frame.width, height: frame.height, rgb })
        }
        other => Err(DecodeError::UnsupportedFormat(other.clone())),
    }
}

/// Integer BT.601 full-range YUV420 (Y + U + V planes) to packed RGB8.
pub fn yuv420_to_rgb8(payload: &[u8], width: u32, height: u32) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let (y_plane, uv) = payload.split_at(w * h);
    let (u_plane, v_plane) = uv.split_at(w * h / 4);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let c = i32::from(y_plane[y * w + x]);
            let d = i32::from(u_plane[(y / 2) * (w / 2) + x / 2]) - 128;
            let e = i32::from(v_plane[(y / 2) * (w / 2) + x / 2]) - 128;
            // Full-range coefficients scaled by 256.
            let r = (c + (359 * e >> 8)).clamp(0, 255) as u8;
            let g = (c - ((88 * d + 183 * e) >> 8)).clamp(0, 255) as u8;
            let b = (c + (454 * d >> 8)).clamp(0, 255) as u8;
            rgb.extend_from_slice(&[r, g, b]);
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;
    use deepsky_camera::model::Origin;

    fn frame(format: PixelFormat, width: u32, height: u32, payload: Vec<u8>) -> PreviewFrame {
        PreviewFrame { payload, width, height, format, timestamp_ns: 0, origin: Origin::Synthetic }
    }

    #[test]
    fn gray_and_rgb_roundtrip() {
        let gray = frame(PixelFormat::Gray8, 2, 1, vec![0, 255]);
        let out = preview_to_rgb8(&gray).unwrap();
        assert_eq!(out.rgb, vec![0, 0, 0, 255, 255, 255]);
        assert!((out.mean_luma() - 127.5).abs() < 1e-9);
        let rgb = frame(PixelFormat::Rgb8, 1, 1, vec![10, 20, 30]);
        assert_eq!(preview_to_rgb8(&rgb).unwrap().rgb, vec![10, 20, 30]);
    }

    #[test]
    fn yuv_black_white_and_size_checks() {
        // 2x2: Y plane then 1 U and 1 V byte (full-range 0=black, 255=white).
        let black = frame(PixelFormat::Yuv420, 2, 2, vec![0, 0, 0, 0, 128, 128]);
        let out = preview_to_rgb8(&black).unwrap();
        assert_eq!(out.rgb, vec![0; 12]);
        let white = frame(PixelFormat::Yuv420, 2, 2, vec![255, 255, 255, 255, 128, 128]);
        assert_eq!(preview_to_rgb8(&white).unwrap().rgb, vec![255; 12]);
        let short = frame(PixelFormat::Yuv420, 2, 2, vec![0; 5]);
        assert!(matches!(preview_to_rgb8(&short), Err(DecodeError::SizeMismatch { .. })));
        let odd = frame(PixelFormat::Yuv420, 3, 2, vec![0; 9]);
        assert!(preview_to_rgb8(&odd).is_err());
    }

    #[test]
    fn rejects_unknown_and_empty() {
        let jpeg = frame(PixelFormat::Jpeg, 2, 2, vec![0; 6]);
        assert!(matches!(preview_to_rgb8(&jpeg), Err(DecodeError::UnsupportedFormat(_))));
        let empty = frame(PixelFormat::Gray8, 0, 0, vec![]);
        assert_eq!(preview_to_rgb8(&empty).unwrap_err(), DecodeError::EmptyFrame);
    }
}
