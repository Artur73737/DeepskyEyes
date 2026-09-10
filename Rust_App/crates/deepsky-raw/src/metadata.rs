//! Metadati RAW trasportati con il frame.
#[derive(Debug, Clone, Default)]
pub struct RawMetadata {
    pub black_level: Option<u16>,
    pub white_level: Option<u16>,
}
