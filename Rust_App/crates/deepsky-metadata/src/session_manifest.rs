//! Manifest sessione machine-readable (README §41).
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionManifest {
    pub schema_version: u32,
    pub project: String,
    pub device: String,
    pub camera_id: String,
    pub frames_requested: u32,
    pub frames_completed: u32,
    pub session_id: String,
    pub started_at: String,
    pub app_version: String,
    pub protocol_version: String,
    pub android_version: Option<String>,
    pub config_version: u32,
    pub configuration: serde_json::Value,
    pub capability_snapshot: serde_json::Value,
    pub frames: Vec<FrameRecord>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub thermal_events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrameRecord {
    /// Portable path relative to the session directory.
    pub filename: String,
    pub size_bytes: u64,
    pub timestamp_ns: u64,
    pub sha256: String,
    pub metadata: super::frame::FrameMetadata,
}

impl Default for SessionManifest {
    fn default() -> Self {
        Self {
            schema_version: MANIFEST_VERSION, project: String::new(), device: String::new(),
            camera_id: String::new(), frames_requested: 0, frames_completed: 0,
            session_id: String::new(), started_at: String::new(), app_version: String::new(),
            protocol_version: String::new(), android_version: None,
            config_version: super::config_version::CONFIG_VERSION,
            configuration: serde_json::Value::Null, capability_snapshot: serde_json::Value::Null,
            frames: vec![], warnings: vec![], errors: vec![], thermal_events: vec![],
        }
    }
}

impl SessionManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MANIFEST_VERSION || self.config_version != super::config_version::CONFIG_VERSION {
            return Err("unsupported manifest/config version".into());
        }
        if self.frames_completed as usize != self.frames.len() || self.frames_completed > self.frames_requested {
            return Err("manifest frame counts inconsistent".into());
        }
        let mut paths = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        for frame in &self.frames {
            if frame.metadata.frame_id.is_empty() || !ids.insert(&frame.metadata.frame_id) || !paths.insert(&frame.filename) {
                return Err("empty/duplicate frame id or filename".into());
            }
            if frame.sha256.len() != 64 || !frame.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid SHA-256 digest".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn version_required_and_roundtrip() {
        let manifest = SessionManifest::default();
        let json = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(serde_json::from_slice::<SessionManifest>(&json).unwrap(), manifest);
        assert!(serde_json::from_str::<SessionManifest>("{}").is_err());
        let mut future = manifest; future.schema_version += 1;
        assert!(future.validate().is_err());
    }
}
