//! PREVIEW: START, STOP, SET_CONFIGURATION.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewCommand {
    StartPreview,
    StopPreview,
    SetPreviewConfiguration { width: u32, height: u32 },
}
