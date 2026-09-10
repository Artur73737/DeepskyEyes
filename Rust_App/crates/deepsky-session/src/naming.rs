//! Naming deterministico: M42_2026-09-10T214512Z_L_0001.dng
pub fn frame_filename(project: &str, timestamp: &str, kind: char, index: u32) -> String {
    format!("{}_{}_{}_{:04}.dng", project, timestamp, kind, index)
}
