//! Runtime contract: send authoritative snapshots; receive user intent.
//! No GPUI types cross this boundary. Call `run` on the main OS thread.
//!
//! Enable `deepsky-ui/desktop` in the desktop binary. Create two standard mpsc
//! channels, keep the snapshot sender and action receiver in the app worker,
//! then call `deepsky_ui::run(initial, snapshot_rx, action_tx)` on the main thread.
//! Send complete, authoritative snapshots (including rejected commands in
//! `message`). The UI never assumes a command succeeded. Coalesce preview
//! updates in the producer to keep the channel bounded in practice.
//! Ranges are inclusive; `None` means the control is unsupported or unknown.
//! Preview revisions must change whenever encoded image contents change.
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    Connect,
    Disconnect,
    SetSource(SourceKind),
    SelectCamera(String),
    SetExposure(u64),
    SetIso(u32),
    SetFocus(f32),
    AutofocusCenter,
    SetWhiteBalance(u32),
    /// Announced WB preset name (e.g. "daylight"); validated, never assumed.
    SetWbPreset(String),
    SetZoom(f32),
    SetRaw(bool),
    SetResolution(u32, u32),
    SetLocked(bool),
    SetFrameCount(u32),
    StartSequence,
    CaptureOne,
    PauseSequence,
    ResumeSequence,
    StopSequence,
    SetFrameType(FrameType),
    RefreshSessions,
    OpenSession(String),
    RefreshDiagnostics,
    /// UI-local intent: opens a native directory picker, then emits SetDestination.
    ChooseDestination,
    SetDestination(String),
    SetAutoSave(bool),
    Shutdown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameType {
    #[default]
    Light,
    Dark,
    Flat,
    Bias,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SequenceStatus {
    #[default]
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Clone, Debug)]
pub struct CameraChoice {
    pub id: String,
    pub label: String,
}

/// Which camera provides frames. Phone = the Pixel over ADB/USB transport.
/// Simulator = the explicit synthetic generator, never hardware data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceKind {
    #[default]
    Phone,
    Simulator,
}
#[derive(Clone, Debug)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub frames: u32,
    pub status: String,
}
/// Encoded PNG/JPEG preview supplied by the preview pipeline, never RAW data.
#[derive(Clone, Debug)]
pub struct PreviewImage {
    pub revision: u64,
    pub bytes: Arc<[u8]>,
}

#[derive(Clone, Debug)]
pub struct UiSnapshot {
    pub connected: bool,
    pub source: SourceKind,
    pub device: String,
    pub camera_id: String,
    pub cameras: Vec<CameraChoice>,
    pub sensor: String,
    pub hardware_level: String,
    pub resolutions: Vec<(u32, u32)>,
    pub resolution: (u32, u32),
    pub raw_supported: bool,
    /// RAW requested for the next configure (pending intent). The applied
    /// stream format stays visible in diagnostics/statistics.
    pub raw_enabled: bool,
    pub locked: bool,
    pub exposure_ns: u64,
    pub applied_exposure_ns: Option<u64>,
    pub exposure_range_ns: Option<(u64, u64)>,
    pub iso: u32,
    pub applied_iso: Option<u32>,
    pub iso_range: Option<(u32, u32)>,
    pub focus_diopters: f32,
    pub focus_range: Option<(f32, f32)>,
    pub white_balance_kelvin: u32,
    pub manual_white_balance: bool,
    /// Pending WB preset; announced preset names the device supports.
    pub wb_preset: String,
    pub wb_presets: Vec<String>,
    pub zoom: f32,
    pub zoom_range: Option<(f32, f32)>,
    pub frames_done: u32,
    pub frames_total: u32,
    pub sequence: SequenceStatus,
    /// Live exposure progress of the frame currently exposing, 0.0..=1.0.
    /// None when no frame is exposing. The UI derives seconds from this.
    pub frame_progress: Option<f32>,
    /// Host-side elapsed request time, not a sensor timestamp.
    pub frame_elapsed_s: Option<f32>,
    pub frame_exposure_s: Option<f32>,
    pub frame_type: FrameType,
    pub preview: Option<PreviewImage>,
    pub histogram: Vec<u32>,
    pub statistics: Vec<(String, String)>,
    pub diagnostics: Vec<(String, String)>,
    pub sessions: Vec<SessionSummary>,
    pub rx_mbps: f64,
    pub tx_mbps: f64,
    pub dropped_raw: u64,
    pub dropped_preview: u64,
    pub storage_free: String,
    pub thermal: String,
    pub destination: String,
    pub auto_save: bool,
    pub message: String,
}
impl Default for UiSnapshot {
    fn default() -> Self {
        Self {
            connected: false,
            source: SourceKind::Phone,
            device: "No device connected".into(),
            camera_id: String::new(),
            cameras: vec![],
            sensor: "Unavailable".into(),
            hardware_level: "Unavailable".into(),
            resolutions: vec![],
            resolution: (0, 0),
            raw_supported: false,
            raw_enabled: false,
            locked: false,
            exposure_ns: 15_000_000_000,
            applied_exposure_ns: None,
            exposure_range_ns: None,
            iso: 800,
            applied_iso: None,
            iso_range: None,
            focus_diopters: 0.,
            focus_range: None,
            white_balance_kelvin: 5000,
            manual_white_balance: false,
            wb_preset: "".into(),
            wb_presets: vec![],
            zoom: 1.,
            zoom_range: None,
            frames_done: 0,
            frames_total: 300,
            sequence: SequenceStatus::Idle,
            frame_progress: None,
            frame_elapsed_s: None,
            frame_exposure_s: None,
            frame_type: FrameType::Light,
            preview: None,
            histogram: vec![],
            statistics: vec![],
            diagnostics: vec![],
            sessions: vec![],
            rx_mbps: 0.,
            tx_mbps: 0.,
            dropped_raw: 0,
            dropped_preview: 0,
            storage_free: "Unavailable".into(),
            thermal: "Unavailable".into(),
            destination: String::new(),
            auto_save: false,
            message: String::new(),
        }
    }
}
