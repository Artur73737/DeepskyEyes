//! Fake RAW/metadata/timing.
pub fn fake_raw_bytes(width: u32, height: u32) -> Vec<u16> {
    vec![0; (width as usize) * (height as usize)]
}
