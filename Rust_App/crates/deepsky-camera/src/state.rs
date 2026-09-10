//! Camera lifecycle: CLOSED/OPENING/OPEN/CONFIGURING/READY/CAPTURING (README §49).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraState {
    #[default]
    Closed,
    Opening,
    Open,
    Configuring,
    Ready,
    Capturing,
    Error,
    Disconnected,
    Recovering,
}
