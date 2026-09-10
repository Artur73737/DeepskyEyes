//! Scrittura DNG desktop (controparte di DngCreator Android).
pub struct DngWriter;

impl DngWriter {
    /// Never label bare samples as DNG or invent sensor color calibration.
    pub fn write(raw: &super::raw_frame::RawFrame) -> std::io::Result<Vec<u8>> {
        raw.validate()?;
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported,
            "DNG needs sensor CFA and color calibration; save to_le_bytes() as .raw16 with metadata"))
    }
}
