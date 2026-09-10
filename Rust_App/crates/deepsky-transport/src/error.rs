//! Errori transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    NotConnected,
    Io,
    Timeout,
    Protocol,
}
