//! Failure injection: USB disconnect, crash, disk full, timeout... (README §83).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedFailure {
    UsbDisconnect,
    AndroidCrash,
    DiskFull,
    TransportTimeout,
    ThermalWarning,
}
