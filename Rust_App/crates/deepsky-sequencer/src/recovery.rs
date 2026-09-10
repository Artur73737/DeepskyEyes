//! Recovery sequenza dopo errore/reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    ResumeIfSafe,
    Abort,
}
