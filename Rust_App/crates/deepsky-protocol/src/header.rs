//! Header logico: version, type, flags, request_id, payload_len, seq (README §29).

use crate::version::PROTOCOL_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub protocol_version: u16,
    pub message_type: u16,
    pub flags: u16,
    pub request_id: u32,
    pub payload_length: u32,
    pub sequence_number: u64,
}

impl Header {
    pub fn new(message_type: u16, request_id: u32, payload_length: u32, sequence_number: u64) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            message_type,
            flags: 0,
            request_id,
            payload_length,
            sequence_number,
        }
    }
}
