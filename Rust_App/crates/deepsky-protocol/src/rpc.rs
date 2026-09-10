//! Provisional RPC contract; freeze only after hardware validation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const REQUEST: u16 = 1;
pub const RESPONSE: u16 = 2;
pub const EVENT: u16 = 3;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub method: String,
    #[serde(default)] pub params: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    Ok { result: Value },
    Error { code: String, message: String },
}
impl Reply {
    pub fn result(self) -> Result<Value, String> {
        match self { Self::Ok { result } => Ok(result), Self::Error { code, message } => Err(format!("{code}: {message}")) }
    }
}
