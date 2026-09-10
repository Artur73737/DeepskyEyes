//! Logical vs physical cameras (README §50).
#[derive(Debug, Clone, Default)]
pub struct LogicalCamera {
    pub logical_id: String,
    pub physical_ids: Vec<String>,
}
