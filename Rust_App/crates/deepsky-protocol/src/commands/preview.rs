//! PREVIEW: START, STOP, SET_CONFIGURATION.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PreviewCommand {
    StartPreview,
    StopPreview,
    SetPreviewConfiguration { width: u32, height: u32 },
}
