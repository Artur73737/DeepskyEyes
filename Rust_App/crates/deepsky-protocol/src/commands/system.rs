//! SYSTEM: HELLO, GET_VERSION, GET_STATUS, PING, GET_TIME.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCommand {
    Hello,
    GetVersion,
    GetStatus,
    Ping { client_timestamp_ns: u64 },
    GetTime,
}
