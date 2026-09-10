//! Controllo esposizione: SENSOR_EXPOSURE_TIME.
#[derive(Debug, Clone, Copy)]
pub struct ExposureControl {
    pub requested_ns: u64,
    pub applied_ns: Option<u64>,
    pub reported_ns: Option<u64>,
}
