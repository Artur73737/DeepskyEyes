//! Testkit: simulator + mock Android (README §88-89).

pub mod simulator;
pub mod mock_android;
pub mod fake_raw;
pub mod failure_injection;
pub mod fixtures;
pub use simulator::{SimulatorBackend, TimingMode, SYNTHETIC_CAMERA_ID};
