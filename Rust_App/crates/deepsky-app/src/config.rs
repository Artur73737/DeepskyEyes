//! Config app versionata.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config_version: u32,
    pub session_root: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { config_version: 1, session_root: "sessions".to_string() }
    }
}
