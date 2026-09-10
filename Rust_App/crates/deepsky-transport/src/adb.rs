//! ADB transport — sviluppo iniziale (README §27, §85).
use crate::{error::TransportError, transport_trait::Transport};

pub struct AdbTransport {
    pub serial: Option<String>,
    connected: bool,
}

impl AdbTransport {
    pub fn new(serial: Option<String>) -> Self {
        Self { serial, connected: false }
    }
}

impl Transport for AdbTransport {
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
