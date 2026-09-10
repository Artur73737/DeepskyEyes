//! Naming deterministico: M42_2026-09-10T214512Z_L_0001.raw16 (o .dng da camera).
//! Solo caratteri portabili; la collisione non viene mai sovrascritta (vedi storage).
pub fn frame_filename(project: &str, timestamp: &str, kind: char, index: u32, extension: &str) -> String {
    let safe_project: String = project
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    format!("{}_{}_{}_{:04}.{}", safe_project, timestamp, kind, index, extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_and_portable() {
        let name = frame_filename("M42 Test!", "2026-09-10T214512Z", 'L', 1, "raw16");
        assert_eq!(name, "M42_Test__2026-09-10T214512Z_L_0001.raw16");
        assert!(!name.contains([' ', '!']));
    }
}
