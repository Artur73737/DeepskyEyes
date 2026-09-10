//! Errori sequencer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequencerError {
    ValidationFailed(&'static str),
    Aborted,
}
