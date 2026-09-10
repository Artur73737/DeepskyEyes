//! Discovery summary built ONLY from announced `CameraCapabilities` snapshots.
//! No IDs, roles or sizes are invented: unknown stays unknown.
use crate::{
    lens::{from_role, LensType},
    model::CameraCapabilities,
};

#[derive(Debug, Clone, Default)]
pub struct DiscoveredCamera {
    pub camera_id: String,
    pub physical_ids: Vec<String>,
    /// "back" / "front" da LENS_FACING (serve a identify() in lens.rs).
    pub lens_facing: String,
    /// Focale equivalente in mm se nota (serve a identify()).
    pub focal_equiv_mm: Option<f32>,
    /// Ruolo da `lens_role` annunciato, mai indovinato.
    pub lens: LensType,
    pub raw_supported: bool,
    pub manual_sensor: bool,
    pub stream_count: usize,
}

/// Summarize snapshots for UI lists and the CLI. Every field traces back to
/// announced evidence; devices that report nothing usable are still listed
/// with `Unknown` / false rather than dropped silently.
pub fn summarize(caps: &[CameraCapabilities]) -> Vec<DiscoveredCamera> {
    caps.iter()
        .map(|c| DiscoveredCamera {
            camera_id: c.camera_id.clone(),
            physical_ids: c.physical_cameras.iter().map(|p| p.id.clone()).collect(),
            lens_facing: c.lens_facing.clone().unwrap_or_default(),
            focal_equiv_mm: None,
            lens: c.lens_role.as_deref().map(from_role).unwrap_or(LensType::Unknown),
            raw_supported: c.raw == Some(true),
            manual_sensor: c.manual_sensor == Some(true),
            stream_count: c.streams.len(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Origin;

    #[test]
    fn unknown_stays_unknown() {
        let caps = CameraCapabilities {
            camera_id: "0".into(),
            identity: "SYNTHETIC".into(),
            origin: Some(Origin::Synthetic),
            ..Default::default()
        };
        let summary = summarize(&[caps]);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].lens, LensType::Unknown);
        assert!(!summary[0].raw_supported);
    }

    #[test]
    fn announced_role_maps() {
        let caps = CameraCapabilities {
            camera_id: "0".into(),
            identity: "SYNTHETIC".into(),
            origin: Some(Origin::Synthetic),
            lens_role: Some("telephoto".into()),
            raw: Some(true),
            ..Default::default()
        };
        let summary = summarize(&[caps]);
        assert_eq!(summary[0].lens, LensType::Telephoto);
        assert!(summary[0].raw_supported);
    }
}
