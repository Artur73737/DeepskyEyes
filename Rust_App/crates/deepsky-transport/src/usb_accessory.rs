//! USB accessory transport — production candidate, NOT yet validated (README §27).
//! Every operation explicitly reports Unsupported until AOA hardware validation
//! lands. Nothing here pretends a connection succeeds.
use crate::{error::TransportError, transport_trait::Transport};

pub struct UsbAccessoryTransport;

impl UsbAccessoryTransport {
    pub fn new() -> Self {
        Self
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
        Err(TransportError::Unsupported)
    }
    fn receive(&mut self) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Unsupported)
    }
    fn disconnect(&mut self) -> Result<(), TransportError> {
        Err(TransportError::Unsupported)
    }
}
