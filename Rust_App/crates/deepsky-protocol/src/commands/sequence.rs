//! SEQUENCE: CREATE, START, PAUSE, RESUME, STOP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceCommand {
    CreateSequence { frames: u32, exposure_ns: u64 },
    StartSequence { sequence_id: String },
    PauseSequence { sequence_id: String },
    ResumeSequence { sequence_id: String },
    StopSequence { sequence_id: String },
}
