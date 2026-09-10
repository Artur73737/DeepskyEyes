//! Tipi wire condivisi.

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CameraId(pub String);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExposureNs(pub u64);

/// Ottica selezionabile (README §50): il mapping ID→lens viene dal discovery.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LensType {
    #[default]
    Unknown,
    MainWide,
    Ultrawide,
    Telephoto,
    Front,
}

impl LensType {
    pub fn label(self) -> &'static str {
        match self {
            LensType::Unknown => "unknown",
            LensType::MainWide => "wide",
            LensType::Ultrawide => "ultrawide",
            LensType::Telephoto => "telephoto",
            LensType::Front => "front",
        }
    }
}
