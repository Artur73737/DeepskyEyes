//! Mock transport per integration test (README §79).
use crate::{error::TransportError, transport_trait::Transport};
use std::collections::VecDeque;

#[derive(Default)]
pub struct MockTransport {
    pub inbox: VecDeque<Vec<u8>>,
    pub outbox: Vec<Vec<u8>>,
    connected: bool,
}

impl MockTransport {
    pub fn push_incoming(&mut self, data: Vec<u8>) {
        self.inbox.push_back(data);
    }
}

impl Transport for MockTransport {
    fn connect(&mut self) -> Result<(), TransportError> {
        self.connected = true;
        Ok(())
    }
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.outbox.push(data.to_vec());
        Ok(())
    }
    fn receive(&mut self) -> Result<Vec<u8>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        Ok(self.inbox.pop_front().unwrap_or_default())
    }
    fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }
}
