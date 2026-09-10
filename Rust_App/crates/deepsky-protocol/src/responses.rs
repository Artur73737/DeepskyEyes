//! Risposte request/response con requested/applied/reported (README §32, §106).

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub request_id: u32,
    pub status: ResponseStatus,
    pub requested_ns: Option<u64>,
    pub applied_ns: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Ok,
    Clamped,
    Error,
}
