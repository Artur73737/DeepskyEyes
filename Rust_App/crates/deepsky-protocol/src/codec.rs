//! UTF-8 JSON inside bounded binary frames.
use serde::{de::DeserializeOwned, Serialize};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError(pub &'static str);
impl std::fmt::Display for CodecError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.0) } }
impl std::error::Error for CodecError {}
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> { serde_json::to_vec(value).map_err(|_| CodecError("JSON encoding failed")) }
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> { serde_json::from_slice(bytes).map_err(|_| CodecError("invalid JSON payload")) }
