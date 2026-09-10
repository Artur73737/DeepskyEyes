//! Honest SYNTHETIC fixture: serialized simulator capabilities, never a Pixel dump.
use crate::simulator::SimulatorBackend;

/// Canonical JSON of the deterministic synthetic capabilities.
pub fn synthetic_capabilities_json() -> String {
    serde_json::to_string_pretty(&SimulatorBackend::capabilities())
        .expect("synthetic capabilities serialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_parses_and_stays_synthetic() {
        let caps: deepsky_camera::model::CameraCapabilities =
            serde_json::from_str(&synthetic_capabilities_json()).unwrap();
        assert_eq!(caps.origin, Some(deepsky_camera::model::Origin::Synthetic));
        assert!(caps.identity.contains("SYNTHETIC"));
    }
}
