//! Risoluzione / stream size (README §11).
//!
//! Mai hard-codare "12MP" o "50MP": la lista supportata viene da
//! `StreamConfigurationMap` (dump Android) e qui si seleziona soltanto.
//! Il toggle Pixel "12.5MP binned vs 50MP full-res" (Pro settings) è un caso
//! particolare di questa selezione, non un valore cablato.

/// Singolo output supportato scoperto sul device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamSize {
    /// Larghezza in pixel (es. 4080 una volta misurata, mai assunta).
    pub width: u32,
    /// Altezza in pixel (es. 3072 una volta misurata, mai assunta).
    pub height: u32,
    /// Formato come codice locale legacy (0=RAW_SENSOR, 1=YUV_420_888, 2=JPEG,
    /// 3=PRIVATE). Il percorso reale di acquisizione usa model::PixelFormat.
    pub format: u8,
    /// true se è modalità binning/ridotta, false se full-res (dal dump).
    pub binned: bool,
}

/// Selezione risoluzione con requested/applied/reported (README §106).
#[derive(Debug, Clone, Default)]
pub struct ResolutionControl {
    /// Tutte le size annunciate dal device per il formato in uso.
    pub available: Vec<StreamSize>,
    pub requested: Option<StreamSize>,
    pub applied: Option<StreamSize>,
    pub reported: Option<StreamSize>,
}

impl ResolutionControl {
    pub fn new(available: Vec<StreamSize>) -> Self {
        Self { available, requested: None, applied: None, reported: None }
    }

    /// La size è tra quelle annunciate? (Niente fallback silenzioso, §108.)
    pub fn is_supported(&self, size: &StreamSize) -> bool {
        self.available.contains(size)
    }

    /// Richiede una size; errore se non supportata.
    pub fn request(&mut self, size: StreamSize) -> Result<(), StreamSize> {
        if self.is_supported(&size) {
            self.requested = Some(size);
            Ok(())
        } else {
            Err(size)
        }
    }
}
