//! Blocking socket I/O belongs on an application worker, never the UI thread.
use crate::{Transport, TransportError};
use deepsky_protocol::framing::{decode_header, Packet, HEADER_LEN, CHECKSUM_LEN};
use std::{io::{Read, Write}, net::{TcpStream, ToSocketAddrs}, time::Duration};
pub struct TcpTransport { pub addr: String, stream: Option<TcpStream>, pub timeout: Duration }
impl TcpTransport {
    pub fn new(addr: impl Into<String>) -> Self { Self { addr: addr.into(), stream: None, timeout: Duration::from_secs(120) } }
    pub fn from_stream(stream: TcpStream) -> Result<Self, TransportError> {
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(120)))?;
        stream.set_nodelay(true)?;
        Ok(Self { addr: stream.peer_addr()?.to_string(), stream: Some(stream), timeout: Duration::from_secs(120) })
    }
}
pub fn read_packet(reader: &mut impl Read) -> Result<Vec<u8>, TransportError> {
    let mut header = [0; HEADER_LEN]; reader.read_exact(&mut header)?;
    let decoded = decode_header(&header).map_err(|_| TransportError::Protocol)?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + decoded.payload_length as usize + CHECKSUM_LEN);
    bytes.extend_from_slice(&header);
    bytes.resize(HEADER_LEN + decoded.payload_length as usize + CHECKSUM_LEN, 0);
    reader.read_exact(&mut bytes[HEADER_LEN..])?;
    Packet::decode(&bytes).map_err(|_| TransportError::Protocol)?;
    Ok(bytes)
}
impl Transport for TcpTransport {
    fn connect(&mut self) -> Result<(), TransportError> {
        if self.stream.is_some() { return Ok(()); }
        let mut last = TransportError::Io;
        for address in self.addr.to_socket_addrs()? {
            match TcpStream::connect_timeout(&address, Duration::from_secs(5)) {
                Ok(stream) => { stream.set_read_timeout(Some(self.timeout))?; stream.set_write_timeout(Some(self.timeout))?; stream.set_nodelay(true)?; self.stream = Some(stream); return Ok(()); }
                Err(e) => last = e.into(),
            }
        } Err(last)
    }
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let result = self.stream.as_mut().ok_or(TransportError::NotConnected)?.write_all(data).map_err(Into::into);
        if result.is_err() { self.stream = None; } result
    }
    fn receive(&mut self) -> Result<Vec<u8>, TransportError> {
        let result = read_packet(self.stream.as_mut().ok_or(TransportError::NotConnected)?);
        // Partial reads cannot safely restart framing: invalidate this connection.
        if result.is_err() { self.stream = None; } result
    }
    fn disconnect(&mut self) -> Result<(), TransportError> { self.stream.take(); Ok(()) }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn fragmented_frame() {
        let frame = Packet::new(1, 2, 3, vec![9;100]).unwrap().encode().unwrap();
        struct Fragmented<'a>(&'a [u8]);
        impl Read for Fragmented<'_> { fn read(&mut self,b:&mut [u8])->std::io::Result<usize>{ let n=b.len().min(3).min(self.0.len()); b[..n].copy_from_slice(&self.0[..n]); self.0=&self.0[n..]; Ok(n) } }
        assert_eq!(read_packet(&mut Fragmented(&frame)).unwrap(),frame);
    }
}
