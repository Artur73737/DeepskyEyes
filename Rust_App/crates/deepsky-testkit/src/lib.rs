//! Testkit: explicit SYNTHETIC simulator for development before/besides hardware (README §88-89).
//! Nothing here claims to be a Pixel measurement; see SYNTHETIC_* markers.

pub mod simulator;
pub mod fake_raw;
pub mod failure_injection;
pub mod fixtures;
pub use simulator::{SimulatorBackend, TimingMode, SYNTHETIC_CAMERA_ID};
