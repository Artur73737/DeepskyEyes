//! Streaming SHA-256 of exact persisted bytes.
use sha2::{Digest, Sha256};
use std::io::{self, Read};
pub fn sha256_hex(data: &[u8]) -> String { format!("{:x}", Sha256::digest(data)) }
pub fn sha256_reader(mut reader: impl Read) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = match reader.read(&mut buffer) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if n == 0 { break; }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn known_vectors() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256_reader(&b"abc"[..]).unwrap(), sha256_hex(b"abc"));
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }
}
