//! Trait Transport — implementazioni: Adb, UsbAccessory, Tcp.

use crate::error::TransportError;

/// Astrazione async minimale (senza runtime esterno nello skeleton).
pub trait Transport {
    fn connect(&mut self) -> Result<(), TransportError>;
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Vec<u8>, TransportError>;
    fn disconnect(&mut self) -> Result<(), TransportError>;
}
