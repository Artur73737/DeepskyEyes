//! Selettore risoluzione: solo size annunciate dal device (mai hard-codare 12/50MP).
//! Il toggle Pixel "12.5MP binned vs 50MP full-res" è una selezione da questa lista.
pub struct ResolutionControls {
    /// Coppie (width, height) annunciate via StreamConfigurationMap.
    pub available: Vec<(u32, u32)>,
    pub selected: Option<(u32, u32)>,
    /// true se la size selezionata non è tra le annunciate (clamp da mostrare).
    pub clamped: bool,
}
