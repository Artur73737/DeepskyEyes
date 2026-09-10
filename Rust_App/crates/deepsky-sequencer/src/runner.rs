//! Real sequence driver: owns NOTHING but progress law.
//!
//! The caller performs capture/storage and reports back. Frames are handed out
//! by `next_frame` and count only after the caller confirms durable storage
//! (`frame_committed`). Retries draw from a lifetime budget that successful
//! frames never reset. Recovery from failure requires caller-supplied evidence
//! — never blind resumption.
use super::{
    errors::SequencerError,
    progress::SequenceProgress,
    recovery::{RecoveryEvidence, RetryPolicy},
    state_machine::SequenceState,
};

pub struct SequenceRunner {
    state: SequenceState,
    total: u32,
    done: u32,
    in_flight: Option<u32>,
    retries_used: u32,
    policy: RetryPolicy,
}

impl SequenceRunner {
    pub fn new(total: u32, policy: RetryPolicy) -> Self {
        Self { state: SequenceState::Idle, total, done: 0, in_flight: None, retries_used: 0, policy }
    }

    pub fn state(&self) -> SequenceState {
        self.state
    }

    pub fn progress(&self) -> SequenceProgress {
        SequenceProgress { done: self.done, total: self.total }
    }

    pub fn retries_used(&self) -> u32 {
        self.retries_used
    }

    /// IDLE -> RUNNING. Pull work with `next_frame`.
    pub fn start(&mut self) -> Result<(), SequencerError> {
        if self.state != SequenceState::Idle || self.total == 0 {
            return Err(SequencerError::InvalidState);
        }
        self.state = SequenceState::Running;
        Ok(())
    }

    /// Hand out the next frame index, or None when the run is complete
    /// (-> COMPLETED). Only callable while RUNNING with nothing outstanding.
    pub fn next_frame(&mut self) -> Result<Option<u32>, SequencerError> {
        if self.state != SequenceState::Running || self.in_flight.is_some() {
            return Err(SequencerError::InvalidState);
        }
        if self.done >= self.total {
            self.state = SequenceState::Completed;
            return Ok(None);
        }
        self.in_flight = Some(self.done);
        Ok(Some(self.done))
    }

    /// Commit after durable storage ack.
    pub fn frame_committed(&mut self, frame: u32) -> Result<(), SequencerError> {
        if self.state != SequenceState::Running || self.in_flight != Some(frame) {
            return Err(SequencerError::InvalidAcknowledgment);
        }
        self.in_flight = None;
        self.done += 1;
        Ok(())
    }

    /// A capture/storage attempt failed. Retryable problems (timeouts, I/O)
    /// consume the lifetime budget; anything else fails the run (-> ERROR).
    pub fn frame_failed(&mut self, frame: u32, retryable: bool) -> Result<(), SequencerError> {
        if self.state != SequenceState::Running || self.in_flight != Some(frame) {
            return Err(SequencerError::InvalidAcknowledgment);
        }
        self.in_flight = None;
        if !retryable {
            self.state = SequenceState::Error;
            return Ok(());
        }
        if self.retries_used >= self.policy.max_retries {
            self.state = SequenceState::Error;
            return Err(SequencerError::RetryExhausted);
        }
        self.retries_used += 1;
        Ok(())
    }

    /// Pause between frames (nothing outstanding).
    pub fn pause(&mut self) -> Result<(), SequencerError> {
        if self.state != SequenceState::Running || self.in_flight.is_some() {
            return Err(SequencerError::InvalidState);
        }
        self.state = SequenceState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), SequencerError> {
        if self.state != SequenceState::Paused {
            return Err(SequencerError::InvalidState);
        }
        self.state = SequenceState::Running;
        Ok(())
    }

    /// Abort from any active state with nothing outstanding.
    pub fn stop(&mut self) -> Result<(), SequencerError> {
        match self.state {
            SequenceState::Running | SequenceState::Paused | SequenceState::Error
                if self.in_flight.is_none() =>
            {
                self.state = SequenceState::Stopped;
                Ok(())
            }
            _ => Err(SequencerError::InvalidState),
        }
    }

    /// Resume an ERRORED run only with reconciled evidence. The evidence must
    /// name exactly the frames already committed — otherwise RecoveryUnsafe.
    pub fn recover(&mut self, evidence: RecoveryEvidence) -> Result<(), SequencerError> {
        if self.state != SequenceState::Error || self.in_flight.is_some() {
            return Err(SequencerError::InvalidState);
        }
        if !(evidence.camera_idle && evidence.storage_reconciled && evidence.last_committed_frame == self.done) {
            return Err(SequencerError::RecoveryUnsafe);
        }
        self.state = SequenceState::Running;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state_machine::SequenceState;

    fn runner(total: u32) -> SequenceRunner {
        SequenceRunner::new(total, RetryPolicy { max_retries: 2, backoff_ms: 0 })
    }

    fn commit_all(total: u32) -> SequenceRunner {
        let mut r = runner(total);
        r.start().unwrap();
        for _ in 0..total {
            let frame = r.next_frame().unwrap().expect("frame due");
            r.frame_committed(frame).unwrap();
        }
        assert_eq!(r.next_frame().unwrap(), None);
        r
    }

    #[test]
    fn completes_only_on_storage_ack() {
        let r = commit_all(3);
        assert_eq!(r.state(), SequenceState::Completed);
        assert_eq!(r.progress().done, 3);
        // Terminal states reject further work.
        let mut r = r;
        assert_eq!(r.next_frame().unwrap_err(), SequencerError::InvalidState);
    }

    #[test]
    fn wrong_frame_ack_rejected() {
        let mut r = runner(2);
        r.start().unwrap();
        r.next_frame().unwrap();
        assert_eq!(r.frame_committed(1).unwrap_err(), SequencerError::InvalidAcknowledgment);
        assert_eq!(r.frame_failed(1, true).unwrap_err(), SequencerError::InvalidAcknowledgment);
        assert_eq!(r.state(), SequenceState::Running);
    }

    #[test]
    fn retry_budget_is_lifetime() {
        let mut r = runner(3);
        r.start().unwrap();
        r.next_frame().unwrap();
        r.frame_failed(0, true).unwrap();
        r.next_frame().unwrap();
        r.frame_committed(0).unwrap();
        // One retry spent on frame 0; only one remains for the whole run.
        r.next_frame().unwrap();
        r.frame_failed(1, true).unwrap();
        r.next_frame().unwrap();
        assert_eq!(r.frame_failed(1, true).unwrap_err(), SequencerError::RetryExhausted);
        assert_eq!(r.state(), SequenceState::Error);
    }

    #[test]
    fn fatal_failure_skips_retry() {
        let mut r = runner(2);
        r.start().unwrap();
        r.next_frame().unwrap();
        r.frame_failed(0, false).unwrap();
        assert_eq!(r.state(), SequenceState::Error);
    }

    #[test]
    fn pause_resume_and_stop() {
        let mut r = runner(2);
        r.start().unwrap();
        r.pause().unwrap();
        assert_eq!(r.state(), SequenceState::Paused);
        assert_eq!(r.next_frame().unwrap_err(), SequencerError::InvalidState);
        r.resume().unwrap();
        r.next_frame().unwrap();
        // No stop or pause with a frame outstanding.
        assert_eq!(r.stop().unwrap_err(), SequencerError::InvalidState);
        assert_eq!(r.pause().unwrap_err(), SequencerError::InvalidState);
        r.frame_committed(0).unwrap();
        r.stop().unwrap();
        assert_eq!(r.state(), SequenceState::Stopped);
    }

    #[test]
    fn recovery_requires_exact_evidence() {
        let mut r = runner(3);
        r.start().unwrap();
        r.next_frame().unwrap();
        r.frame_committed(0).unwrap();
        r.next_frame().unwrap();
        r.frame_failed(1, false).unwrap();
        let stale = RecoveryEvidence { camera_idle: true, storage_reconciled: false, last_committed_frame: 1, elapsed_since_failure_ms: 0 };
        assert_eq!(r.recover(stale).unwrap_err(), SequencerError::RecoveryUnsafe);
        let drifted = RecoveryEvidence { camera_idle: true, storage_reconciled: true, last_committed_frame: 2, elapsed_since_failure_ms: 0 };
        assert_eq!(r.recover(drifted).unwrap_err(), SequencerError::RecoveryUnsafe);
        let good = RecoveryEvidence { camera_idle: true, storage_reconciled: true, last_committed_frame: 1, elapsed_since_failure_ms: 0 };
        r.recover(good).unwrap();
        assert_eq!(r.state(), SequenceState::Running);
        assert_eq!(r.next_frame().unwrap(), Some(1));
    }

    #[test]
    fn empty_plan_and_double_start_rejected() {
        let mut r = runner(0);
        assert_eq!(r.start().unwrap_err(), SequencerError::InvalidState);
        let mut r = runner(1);
        r.start().unwrap();
        assert_eq!(r.start().unwrap_err(), SequencerError::InvalidState);
    }
}
