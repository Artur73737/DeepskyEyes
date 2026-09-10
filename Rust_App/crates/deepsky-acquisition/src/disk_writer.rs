//! Disk writer asincrono (placeholder sync nello skeleton).
pub struct DiskWriter {
    pub root: String,
}

impl DiskWriter {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }
}
