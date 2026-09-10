//! Selezione ottica: wide / ultrawide / telephoto / front (README §50, §50b).
//!
//! Il tipo wire è quello del protocollo; qui vivono discovery-mapping e
//! selezione requested/applied. L'autorità resta il dump sul device
//! (physical IDs + `LENS_FACING` + focali), mai gli ID hard-codati.

pub use deepsky_protocol::wire_types::LensType;

/// Ottica disponibile con il suo camera ID (logico o fisico).
#[derive(Debug, Clone, Default)]
pub struct CameraLens {
    pub camera_id: String,
    pub physical_id: Option<String>,
    pub lens: LensType,
}

/// Selettore: solo ottiche annunciate, niente fallback silenzioso (§108).
#[derive(Debug, Clone, Default)]
pub struct LensSelector {
    pub available: Vec<CameraLens>,
    pub requested: Option<LensType>,
    pub applied: Option<LensType>,
}

impl LensSelector {
    pub fn select(&mut self, lens: LensType) -> Result<(), LensType> {
        if self.available.iter().any(|c| c.lens == lens) {
            self.requested = Some(lens);
            Ok(())
        } else {
            Err(lens)
        }
    }
}

/// Euristica iniziale da facing + focale equivalente.
/// Soglie indicative (Pixel 8 Pro: ultra ~12mm, main ~25mm, tele ~112mm):
/// vanno confermate con il dump, non usate come verità.
pub fn identify(facing: &str, focal_equiv_mm: Option<f32>) -> LensType {
    if facing.eq_ignore_ascii_case("front") {
        return LensType::Front;
    }
    match focal_equiv_mm {
        Some(f) if f < 18.0 => LensType::Ultrawide,
        Some(f) if f < 40.0 => LensType::MainWide,
        Some(f) if f >= 40.0 => LensType::Telephoto,
        _ => LensType::Unknown,
    }
}
