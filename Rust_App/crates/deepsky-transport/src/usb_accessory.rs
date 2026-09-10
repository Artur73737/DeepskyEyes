//! USB accessory transport — candidato produzione (AOA).
use crate::{error::TransportError, transport_trait::Transport};

pub struct UsbAccessoryTransport {
    connected: bool,
}

impl UsbAccessoryTransport {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Default for UsbAccessoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for UsbAccessoryTransport {
    fn connect(&mut self) -> Result<(), TransportError> {
        Err(TransportError::Unsupported)
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
