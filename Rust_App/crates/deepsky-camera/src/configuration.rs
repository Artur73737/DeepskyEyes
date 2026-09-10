//! Configurazione camera con Requested/Applied/Reported (README §106).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState<T> {
    Requested(T),
    Applied(T),
    Reported(T),
}

#[derive(Debug, Clone, Default)]
pub struct CameraConfiguration {
    pub camera_id: String,
    pub exposure_ns_requested: Option<u64>,
    pub exposure_ns_applied: Option<u64>,
    pub sensitivity_requested: Option<u32>,
    pub sensitivity_applied: Option<u32>,
    pub focus_locked: bool,
    /// Risoluzione selezionata (w, h) — solo da lista annunciata ( §11b).
    pub resolution_requested: Option<(u32, u32)>,
    pub resolution_applied: Option<(u32, u32)>,
    /// Zoom ×1000 (1000=1.0x) — requested/applied.
    pub zoom_ratio_x1000_requested: Option<u32>,
    pub zoom_ratio_x1000_applied: Option<u32>,
    /// Ottica selezionata (wide/ultrawide/telephoto/front) — requested/applied (§50b).
    pub lens_requested: Option<deepsky_protocol::wire_types::LensType>,
    pub lens_applied: Option<deepsky_protocol::wire_types::LensType>,
}
