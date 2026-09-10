//! Recovery sequenza dopo errore/reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    ResumeIfSafe,
    Abort,
}

/// Lifetime budget for a run; successful frames do not reset it.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy { pub max_retries: u32, pub backoff_ms: u64 }
impl Default for RetryPolicy { fn default() -> Self { Self { max_retries: 3, backoff_ms: 1000 } } }
/// Supplied only after the caller reconciles camera and durable storage.
#[derive(Debug, Clone, Copy)]
pub struct RecoveryEvidence {
    pub camera_idle: bool,
    pub storage_reconciled: bool,
    pub last_committed_frame: u32,
    pub elapsed_since_failure_ms: u64,
}
