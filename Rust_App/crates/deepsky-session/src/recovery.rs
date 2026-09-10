//! Crash recovery: scan/verifica/ricostruzione/resume senza overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDecision {
    Resume,
    StartFresh,
    Abort,
}
