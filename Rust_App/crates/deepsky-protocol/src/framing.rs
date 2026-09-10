//! Framing: length-delimited + checksum opzionale (README §29).

/// Lunghezza header in byte (placeholder, wire reale da definire in protocol-v1).
pub const HEADER_LEN: usize = 20;

pub fn encode_frame(header_bytes: &[u8], payload: &[u8], checksum: Option<u32>) -> Vec<u8> {
    let mut out = Vec::with_capacity(header_bytes.len() + payload.len() + 4);
    out.extend_from_slice(header_bytes);
    out.extend_from_slice(payload);
    if let Some(c) = checksum {
        out.extend_from_slice(&c.to_le_bytes());
    }
    out
}
