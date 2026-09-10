//! Scrittura DNG desktop (controparte di DngCreator Android).
pub struct DngWriter;

impl DngWriter {
    pub fn write_stub(_raw: &super::raw_frame::RawFrame) -> Vec<u8> {
        Vec::new()
    }
}
