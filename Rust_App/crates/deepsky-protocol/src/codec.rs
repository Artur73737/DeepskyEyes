//! Codec placeholder: encode/decode verso byte (serde reale in Phase 2).

/// Errore codec minimale per lo skeleton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError(pub &'static str);
