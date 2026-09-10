//! STORAGE: GET_FRAME, DELETE_FRAME, GET_METADATA.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum StorageCommand {
    GetFrame { frame_id: String },
    DeleteFrame { frame_id: String },
    GetMetadata { frame_id: String },
}
