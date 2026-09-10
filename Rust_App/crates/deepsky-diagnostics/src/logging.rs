//! Log strutturati con session/sequence/frame/request_id.
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub session_id: String,
    pub sequence_id: String,
    pub frame: u32,
    pub request_id: u32,
    pub message: String,
}
