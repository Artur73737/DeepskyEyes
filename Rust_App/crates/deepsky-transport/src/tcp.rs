//! TCP transport — fallback e test.
use crate::{error::TransportError, transport_trait::Transport};

pub struct TcpTransport {
    pub addr: String,
    connected: bool,
}

impl TcpTransport {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into(), connected: false }
    }
}

impl Transport for TcpTransport {
    fn connect(&mut self) -> Result<(), TransportError> {
        self.connected = true;
        Ok(())
    }
    fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        Ok(())
    }
    fn receive(&mut self) -> Result<Vec<u8>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        Ok(Vec::new())
    }
    fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }
}
