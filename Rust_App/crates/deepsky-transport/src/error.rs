//! Errori transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    NotConnected,
    Io,
    Timeout,
    Protocol,
    Unsupported,
}
impl std::fmt::Display for TransportError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") } }
impl std::error::Error for TransportError {}
impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() { std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::Timeout, std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe => Self::NotConnected, _ => Self::Io }
    }
}
