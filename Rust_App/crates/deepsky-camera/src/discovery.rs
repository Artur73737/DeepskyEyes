//! Discovery: enumerazione ID e stream map (placeholder).
#[derive(Debug, Clone, Default)]
pub struct DiscoveredCamera {
    pub camera_id: String,
    pub physical_ids: Vec<String>,
    /// "back" / "front" da LENS_FACING (serve a identify() in lens.rs).
    pub lens_facing: String,
    /// Focale equivalente in mm se nota (serve a identify()).
    pub focal_equiv_mm: Option<f32>,
}
