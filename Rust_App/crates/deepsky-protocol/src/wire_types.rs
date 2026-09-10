//! Tipi wire condivisi.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExposureNs(pub u64);
