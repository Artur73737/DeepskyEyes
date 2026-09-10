//! Errori sequencer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequencerError {
    ValidationFailed(&'static str),
    Aborted,
    InvalidState,
    InvalidAcknowledgment,
    RetryExhausted,
    RecoveryUnsafe,
    Camera(String),
}

impl std::fmt::Display for SequencerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for SequencerError {}
