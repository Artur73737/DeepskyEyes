//! Statistiche acquisizione: dropped RAW/preview.
#[derive(Debug, Default, Clone, Copy)]
pub struct AcquisitionStats {
    pub dropped_raw: u64,
    pub dropped_preview: u64,
}
