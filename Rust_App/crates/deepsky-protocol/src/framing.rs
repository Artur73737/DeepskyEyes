//! Version 1: little-endian header, bounded payload, SHA-256 integrity.
use crate::{codec::CodecError, header::Header, version::PROTOCOL_VERSION};
use sha2::{Digest, Sha256};
pub const HEADER_LEN: usize = 26;
pub const CHECKSUM_LEN: usize = 32;
pub const MAX_PAYLOAD: usize = 256 * 1024 * 1024;
const MAGIC: &[u8; 4] = b"DSKY";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet { pub header: Header, pub payload: Vec<u8> }
impl Packet {
    pub fn new(message_type: u16, request_id: u32, sequence: u64, payload: Vec<u8>) -> Result<Self, CodecError> {
        if payload.len() > MAX_PAYLOAD { return Err(CodecError("payload too large")); }
        Ok(Self { header: Header::new(message_type, request_id, payload.len() as u32, sequence), payload })
    }
    pub fn encode(&self) -> Result<Vec<u8>, CodecError> {
        if self.header.payload_length as usize != self.payload.len() || self.payload.len() > MAX_PAYLOAD { return Err(CodecError("invalid payload length")); }
        if self.header.protocol_version != PROTOCOL_VERSION || self.header.flags != 0 { return Err(CodecError("unsupported version or flags")); }
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len() + CHECKSUM_LEN);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.header.protocol_version.to_le_bytes());
        bytes.extend_from_slice(&self.header.message_type.to_le_bytes());
        bytes.extend_from_slice(&self.header.flags.to_le_bytes());
        bytes.extend_from_slice(&self.header.request_id.to_le_bytes());
        bytes.extend_from_slice(&self.header.payload_length.to_le_bytes());
        bytes.extend_from_slice(&self.header.sequence_number.to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        let digest = Sha256::digest(&bytes);
        bytes.extend_from_slice(&digest);
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        let header = decode_header(bytes)?;
        let len = HEADER_LEN + header.payload_length as usize;
        if bytes.len() != len + CHECKSUM_LEN { return Err(CodecError("incomplete or trailing frame bytes")); }
        if Sha256::digest(&bytes[..len])[..] != bytes[len..] { return Err(CodecError("checksum mismatch")); }
        Ok(Self { header, payload: bytes[HEADER_LEN..len].to_vec() })
    }
}
pub fn decode_header(bytes: &[u8]) -> Result<Header, CodecError> {
    if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC { return Err(CodecError("invalid frame header")); }
    let header = Header {
        protocol_version: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        message_type: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        flags: u16::from_le_bytes(bytes[8..10].try_into().unwrap()),
        request_id: u32::from_le_bytes(bytes[10..14].try_into().unwrap()),
        payload_length: u32::from_le_bytes(bytes[14..18].try_into().unwrap()),
        sequence_number: u64::from_le_bytes(bytes[18..26].try_into().unwrap()),
    };
    if header.protocol_version != PROTOCOL_VERSION || header.flags != 0 { return Err(CodecError("unsupported version or flags")); }
    if header.payload_length as usize > MAX_PAYLOAD { return Err(CodecError("payload too large")); }
    Ok(header)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn round_trip_and_corruption() {
        let packet = Packet::new(1, 8, 42, vec![1, 2, 3]).unwrap();
        let mut bytes = packet.encode().unwrap();
        assert_eq!(Packet::decode(&bytes).unwrap(), packet);
        bytes[HEADER_LEN] ^= 1;
        assert!(Packet::decode(&bytes).is_err());
    }
    #[test] fn reject_truncated_and_oversized() {
        let bytes = Packet::new(1, 1, 1, vec![]).unwrap().encode().unwrap();
        for n in 0..bytes.len() { assert!(Packet::decode(&bytes[..n]).is_err()); }
        let mut bytes = bytes;
        bytes[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_header(&bytes).is_err());
    }
}
