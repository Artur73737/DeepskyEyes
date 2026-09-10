//! Errori machine-readable (README §33).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    UnsupportedParameter,
    InvalidValue,
    ValueClamped,
    CameraBusy,
    CameraDisconnected,
    SessionConfigurationFailed,
    CaptureFailed,
    RawNotSupported,
    FormatNotSupported,
    SizeNotSupported,
    TransportError,
    Timeout,
    ThermalLimit,
    StorageError,
    ProtocolError,
}
