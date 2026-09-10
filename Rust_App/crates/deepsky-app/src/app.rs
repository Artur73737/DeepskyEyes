//! App root: possiede stato, cablaggio core/UI.
use crate::{config::AppConfig, state::AppStateCore};

pub struct App {
    pub config: AppConfig,
    pub state: AppStateCore,
}

impl App {
    pub fn new() -> Self {
        Self { config: AppConfig::default(), state: AppStateCore::default() }
    }
    pub fn version(&self) -> &'static str {
        "0.1.0"
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
