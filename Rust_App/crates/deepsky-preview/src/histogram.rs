//! Istogramma RGB/luminanza + clipping.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub r: [u32; 256],
    pub g: [u32; 256],
    pub b: [u32; 256],
}

impl Default for Histogram {
    fn default() -> Self {
        Self { r: [0; 256], g: [0; 256], b: [0; 256] }
    }
}

impl Histogram {
    /// 256-bin per channel over packed RGB8.
    pub fn from_rgb8(rgb: &[u8]) -> Self {
        let mut h = Self::default();
        for p in rgb.chunks_exact(3) {
            h.r[p[0] as usize] += 1;
            h.g[p[1] as usize] += 1;
            h.b[p[2] as usize] += 1;
        }
        h
    }

    pub fn total_samples(&self) -> u64 {
        self.r.iter().map(|&v| u64::from(v)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_channels() {
        let h = Histogram::from_rgb8(&[0, 0, 0, 255, 255, 255]);
        assert_eq!(h.r[0], 1);
        assert_eq!(h.r[255], 1);
        assert_eq!(h.total_samples(), 2);
    }
}
