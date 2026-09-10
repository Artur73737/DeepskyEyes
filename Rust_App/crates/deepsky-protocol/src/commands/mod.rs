//! Famiglie comandi (README §30).

pub mod system;
pub mod device;
pub mod camera;
pub mod control;
pub mod preview;
pub mod capture;
pub mod sequence;
pub mod storage;
pub mod diagnostics;

/// Id messaggio per famiglia (placeholder stabili).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageFamily {
    System = 1,
    Device = 2,
    Camera = 3,
    Control = 4,
    Preview = 5,
    Capture = 6,
    Sequence = 7,
    Storage = 8,
    Diagnostics = 9,
}
