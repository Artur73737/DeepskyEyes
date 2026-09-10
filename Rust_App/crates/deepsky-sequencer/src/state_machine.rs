//! State machine: IDLE/VALIDATING/PREPARING/RUNNING/PAUSED/COMPLETING/COMPLETED + ERROR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SequenceState {
    #[default]
    Idle,
    Validating,
    Preparing,
    Running,
    Paused,
    Completing,
    Completed,
    Error,
    Recovering,
    FatalError,
    Stopped,
}
