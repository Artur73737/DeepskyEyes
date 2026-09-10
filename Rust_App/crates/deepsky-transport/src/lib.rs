//! Transport abstraction (README §26). Camera code -> Protocol -> Transport.

pub mod transport_trait;
pub mod error;
pub mod stats;
pub mod adb;
pub mod usb_accessory;
pub mod tcp;
pub mod mock;

pub use transport_trait::Transport;
pub use error::TransportError;
