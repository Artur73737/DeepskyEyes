//! Shared mission-control pipeline: connect -> discover -> open -> configure ->
//! acquire -> durable disk + session manifest. Used by the CLI and the desktop
//! worker. Every number reported comes from the device or the filesystem.
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deepsky_acquisition::disk_writer::{DiskWorker, DiskWriter, WriteJob};
use deepsky_camera::{
    backend::CameraBackend,
    model::{
        CameraCapabilities, CameraError, CameraSelection, CaptureMetadata, CaptureRequest,
        CaptureSettings, ErrorCode, FocusRequest, Origin, PixelFormat, ThermalStatus,
        ValidationPolicy, WhiteBalanceRequest,
    },
};
use deepsky_metadata::{
    config_version::CONFIG_VERSION,
    frame::FrameMetadata,
    session_manifest::{FrameRecord, SessionManifest, MANIFEST_VERSION},
};
use deepsky_preview::decoder::preview_to_rgb8;
use deepsky_protocol::version::PROTOCOL_VERSION;
use deepsky_sequencer::{
    errors::SequencerError,
    plan::SequencePlan,
    recovery::RetryPolicy,
    runner::SequenceRunner,
    state_machine::SequenceState,
    validation::SequenceValidation,
};
use deepsky_session::{calibration::SessionKind, naming::frame_filename, store::SessionStore};

use crate::source::Source;

pub const APP_VERSION: &str = "0.1.0";
/// Adaptive Thermal research points (doc/10): warn at 49 C, refuse at 52 C.
pub const THERMAL_WARN_C: f32 = 49.0;
pub const THERMAL_CRITICAL_C: f32 = 52.0;

#[derive(Debug)]
pub enum ControllerError {
    Camera(CameraError),
    Io(io::Error),
    Sequencer(SequencerError),
    Config(String),
    Thermal(String),
}

impl std::fmt::Display for ControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Camera(e) => write!(f, "camera: {e}"),
            Self::Io(e) => write!(f, "storage: {e}"),
            Self::Sequencer(e) => write!(f, "sequencer: {e}"),
            Self::Config(m) => write!(f, "config: {m}"),
            Self::Thermal(m) => write!(f, "thermal: {m}"),
        }
    }
}
impl std::error::Error for ControllerError {}
impl From<io::Error> for ControllerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<CameraError> for ControllerError {
    fn from(e: CameraError) -> Self {
        Self::Camera(e)
    }
}
impl From<SequencerError> for ControllerError {
    fn from(e: SequencerError) -> Self {
        Self::Sequencer(e)
    }
}

fn config(message: impl Into<String>) -> ControllerError {
    ControllerError::Config(message.into())
}

/// Connect a source and return an authenticated (HELLO-checked) backend.
pub fn connect_source(source: &Source) -> Result<Box<dyn CameraBackend + Send>, ControllerError> {
    Ok(source.connect()?)
}

pub(crate) fn pick_camera(caps: &[CameraCapabilities]) -> Result<&CameraCapabilities, ControllerError> {    caps.iter()
        .find(|c| c.origin == Some(Origin::Device))
        .or_else(|| caps.first())
        .ok_or_else(|| config("device announced zero cameras"))
}

fn raw_stream(caps: &CameraCapabilities) -> Result<deepsky_camera::model::StreamConfiguration, ControllerError> {
    caps.streams
        .iter()
        .find(|s| matches!(s.format, PixelFormat::Raw16Le | PixelFormat::Dng))
        .cloned()
        .ok_or_else(|| {
            ControllerError::Camera(CameraError::new(ErrorCode::Unsupported, "no RAW stream announced"))
        })
}

fn focus_request(caps: &CameraCapabilities, millidiopters: u64, locked: bool) -> Result<FocusRequest, ControllerError> {
    if !caps.af_modes.iter().any(|m| m == "off") {
        return Err(ControllerError::Camera(CameraError::new(
            ErrorCode::Unsupported,
            "manual focus: AF OFF not announced",
        )));
    }
    Ok(FocusRequest::Manual { millidiopters, locked })
}

fn white_balance_request(caps: &CameraCapabilities, kelvin: u64) -> Result<WhiteBalanceRequest, ControllerError> {
    if caps.wb_modes.iter().any(|m| m == "temperature") && caps.wb_kelvin.is_some() {
        return Ok(WhiteBalanceRequest::Temperature { kelvin, tint: None });
    }
    if caps.wb_modes.iter().any(|m| m == "manual") && caps.wb_gain_x1000.is_some() {
        // Explicit neutral selection, reported as such — not a device preset.
        return Ok(WhiteBalanceRequest::Manual { gains_x1000: [1000, 1000, 1000, 1000], transform_millionths: [1_000_000, 0, 0, 0, 1_000_000, 0, 0, 0, 1_000_000] });
    }
    for preset in ["daylight", "cloudy_daylight", "shade", "twilight", "fluorescent", "warm_fluorescent", "incandescent"] {
        if caps.wb_modes.iter().any(|m| m == preset) {
            return Ok(WhiteBalanceRequest::Mode(preset.into()));
        }
    }
    Err(ControllerError::Camera(CameraError::new(ErrorCode::Unsupported, "no fixed white balance announced")))
}

fn processing_minimal(caps: &CameraCapabilities) -> std::collections::BTreeMap<String, String> {
    caps.processing_modes
        .iter()
        .filter(|(_, modes)| modes.iter().any(|m| m == "off"))
        .map(|(k, _)| (k.clone(), "off".to_string()))
        .collect()
}

/// Explicit capture parameters. Every field is validated device-side with the
/// Reject policy; the constructor below never clamps or substitutes.
#[derive(Debug, Clone)]
pub struct CaptureSpec {
    pub exposure_ns: u64,
    pub sensitivity: u32,
    pub focus_millidiopters: u64,
    pub focus_locked: bool,
    pub wb_kelvin: u64,
    pub zoom_x1000: Option<u64>,
    pub stream_override: Option<deepsky_camera::model::StreamConfiguration>,
    /// true = first announced RAW stream; false = first announced non-RAW stream.
    pub raw: bool,
}

/// Build an explicit capture request from announced capabilities only.
pub fn build_request(caps: &CameraCapabilities, spec: &CaptureSpec) -> Result<CaptureRequest, ControllerError> {
    let stream = match (&spec.stream_override, spec.raw) {
        (Some(stream), _) => stream.clone(),
        (None, true) => raw_stream(caps)?,
        (None, false) => caps
            .streams
            .iter()
            .find(|s| !matches!(s.format, PixelFormat::Raw16Le | PixelFormat::Dng))
            .cloned()
            .ok_or_else(|| {
                ControllerError::Camera(CameraError::new(ErrorCode::Unsupported, "no processed stream announced"))
            })?,
    };
    Ok(CaptureRequest {
        request_id: 1,
        selection: CameraSelection { camera_id: caps.camera_id.clone(), physical_id: None },
        settings: CaptureSettings {
            exposure_ns: Some(spec.exposure_ns),
            sensitivity: Some(u64::from(spec.sensitivity)),
            frame_duration_ns: None,
            focus: Some(focus_request(caps, spec.focus_millidiopters, spec.focus_locked)?),
            zoom_x1000: spec.zoom_x1000,
            crop: None,
            stream: Some(stream),
            white_balance: Some(white_balance_request(caps, spec.wb_kelvin)?),
            processing: processing_minimal(caps),
            ois: caps.ois_modes.iter().find(|m| *m == "off").cloned(),
            eis: caps.eis_modes.iter().find(|m| *m == "off").cloned(),
        },
    })
}

pub fn now_unix_ns() -> Result<u64, ControllerError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().min(u128::from(u64::MAX)) as u64)
        .map_err(|_| config("system clock before Unix epoch"))
}

/// UTC stamp `YYYY-MM-DDTHHMMSSZ` from Unix seconds (civil-from-days, no deps).
pub fn unix_to_utc_stamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let sod = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}{:02}{:02}Z", year, m, d, sod / 3_600, sod % 3_600 / 60, sod % 60)
}

fn subdir(kind: SessionKind) -> (&'static str, char) {
    match kind {
        SessionKind::Light => ("lights", 'L'),
        SessionKind::Dark => ("darks", 'D'),
        SessionKind::Flat => ("flats", 'F'),
        SessionKind::Bias => ("bias", 'B'),
        SessionKind::Test => ("test", 'T'),
    }
}

pub(crate) fn check_thermal(status: &ThermalStatus, warnings: &mut Vec<String>) -> Result<(), ControllerError> {
    let critical = status.severity.eq_ignore_ascii_case("critical")
        || status.temperature_c.is_some_and(|t| t >= THERMAL_CRITICAL_C);
    if critical {
        return Err(ControllerError::Thermal(format!(
            "thermal critical ({} / {:?} C): refusing capture",
            status.severity, status.temperature_c
        )));
    }
    if status.severity.eq_ignore_ascii_case("warning")
        || status.temperature_c.is_some_and(|t| t >= THERMAL_WARN_C)
    {
        warnings.push(format!("thermal warning at run start: {} / {:?} C", status.severity, status.temperature_c));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AcquisitionOptions {
    pub project: String,
    pub out_dir: PathBuf,
    pub calibration: SessionKind,
    pub frames: u32,
    pub exposure_ns: u64,
    pub sensitivity: u32,
    pub focus_millidiopters: u64,
    pub wb_kelvin: u64,
    pub delay_ns: u64,
    pub focus_locked: bool,
    pub zoom_x1000: Option<u64>,
    pub stream_override: Option<deepsky_camera::model::StreamConfiguration>,
    pub raw: bool,
}

impl AcquisitionOptions {
    pub fn spec(&self) -> CaptureSpec {
        CaptureSpec {
            exposure_ns: self.exposure_ns,
            sensitivity: self.sensitivity,
            focus_millidiopters: self.focus_millidiopters,
            focus_locked: self.focus_locked,
            wb_kelvin: self.wb_kelvin,
            zoom_x1000: self.zoom_x1000,
            stream_override: self.stream_override.clone(),
            raw: self.raw,
        }
    }
}

impl Default for AcquisitionOptions {
    fn default() -> Self {        Self {
            project: "TEST".into(),
            out_dir: PathBuf::from("sessions"),
            calibration: SessionKind::Test,
            frames: 1,
            exposure_ns: 1_000_000,
            sensitivity: 100,
            focus_millidiopters: 0,
            wb_kelvin: 5_000,
            delay_ns: 0,
            focus_locked: true,
            zoom_x1000: None,
            stream_override: None,
            raw: true,
        }
    }
}

#[derive(Debug)]
pub struct AcquisitionReport {
    pub session_dir: PathBuf,
    pub session_id: String,
    pub frames_committed: u32,
    pub frame_sha256: Vec<String>,
    pub preview_mean_luma: Option<f64>,
    pub warnings: Vec<String>,
    pub origin: Origin,
    pub identity: String,
}

/// Everything needed to acquire frames, without any runner state.
/// Lets the CLI (blocking) and the desktop worker (stepwise) share setup.
pub struct PreparedRun {
    pub outcome: deepsky_camera::model::ConfigurationOutcome,
    pub stream: deepsky_camera::model::StreamConfiguration,
    pub store: SessionStore,
    pub disk: DiskWorker,
    pub session_dir: PathBuf,
    pub session_id: String,
    pub stamp: String,
    pub ctx: FrameCtx,
    pub warnings: Vec<String>,
    pub thermal: ThermalStatus,
    pub payload_bytes: u64,
}

/// Open, configure (Reject policy), preflight, create session + disk worker.
/// Nothing is assumed: every step can fail with the responsible cause.
pub fn prepare_run(
    backend: &mut dyn CameraBackend,
    caps: &CameraCapabilities,
    opts: &AcquisitionOptions,
    now_ns: u64,
) -> Result<PreparedRun, ControllerError> {
    let selection = CameraSelection { camera_id: caps.camera_id.clone(), physical_id: None };
    backend.open(&selection)?;
    let request = build_request(&caps, &opts.spec())?;
    let outcome = backend.configure(&request, ValidationPolicy::Reject)?;

    let stream = outcome.applied.settings.stream.clone().ok_or_else(|| config("configured without stream"))?;
    let payload_bytes = match stream.format {
        PixelFormat::Raw16Le => {
            let count = u64::from(stream.width) * u64::from(stream.height);
            count.checked_mul(2).ok_or_else(|| config("stream size overflow"))?
        }
        PixelFormat::Dng => 256 * 1024 * 1024,
        _ => return Err(config("configured stream is not a RAW format")),
    };

    let mut warnings = Vec::new();
    let thermal = backend.thermal()?;
    check_thermal(&thermal, &mut warnings)?;
    if caps.origin != Some(Origin::Device) {
        warnings.push(format!("origin is {:?}: not a hardware measurement", caps.origin));
    }

    let plan = SequencePlan {
        frames: opts.frames,
        exposure_ns: opts.exposure_ns,
        sensitivity: opts.sensitivity,
        delay_ns: opts.delay_ns,
    };
    let preflight = SequenceValidation {
        camera_connected: true,
        camera_open: true,
        raw_supported: caps.raw == Some(true),
        focus_locked: matches!(
            outcome.applied.settings.focus,
            Some(FocusRequest::Manual { locked: true, .. }) | Some(FocusRequest::Auto { locked: true, .. })
        ),
        storage_ok: true, // proven below by SessionStore::create (never assumed)
        thermal_ok: true,  // proven by check_thermal above
        white_balance_fixed: true, // build_request only emits fixed WB
        transport_ok: true, // proven by the round trips above
        clock_valid: true,  // now_ns was read successfully by the caller
        available_bytes: None, // free space is not queryable portably: no number invented
        bytes_per_frame: payload_bytes.saturating_add(1024 * 1024),
    };
    preflight.validate(&plan, caps, &outcome.applied).map_err(ControllerError::from)?;

    let stamp = unix_to_utc_stamp(now_ns / 1_000_000_000);
    let session_id = format!("sess-{}-{}", now_ns, std::process::id());
    // The session directory itself must not pre-exist (no implicit reuse),
    // but missing parents of the chosen output root are created explicitly.
    std::fs::create_dir_all(&opts.out_dir)?;
    let session_dir = opts.out_dir.join(format!("{}_{}", opts.project, stamp));
    let (dir_name, kind_char) = subdir(opts.calibration);
    let manifest = SessionManifest {
        schema_version: MANIFEST_VERSION,
        project: opts.project.clone(),
        device: caps.identity.clone(),
        camera_id: caps.camera_id.clone(),
        frames_requested: opts.frames,
        frames_completed: 0,
        session_id: session_id.clone(),
        started_at: stamp.clone(),
        app_version: APP_VERSION.into(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        android_version: None, // unknown until the Kotlin side reports it
        config_version: CONFIG_VERSION,
        configuration: serde_json::to_value(&outcome.applied).map_err(|e| config(e.to_string()))?,
        capability_snapshot: serde_json::to_value(caps).map_err(|e| config(e.to_string()))?,
        frames: vec![],
        warnings: warnings.clone(),
        errors: vec![],
        thermal_events: vec![serde_json::json!({
            "at_unix_ns": now_ns,
            "severity": thermal.severity,
            "temperature_c": thermal.temperature_c,
        })],
    };
    let store = SessionStore::create(&session_dir, manifest)?;
    let disk = DiskWriter::new(session_dir.to_string_lossy().as_ref()).start(8, payload_bytes as usize)?;
    let ext = match stream.format {
        PixelFormat::Raw16Le => "raw16",
        PixelFormat::Dng => "dng",
        _ => unreachable!("stream format checked above"),
    }
    .to_string();
    let ctx = FrameCtx {
        dir_name: dir_name.to_string(),
        kind_char,
        project: opts.project.clone(),
        stamp: stamp.clone(),
        ext,
    };
    Ok(PreparedRun { outcome, stream, store, disk, session_dir, session_id, stamp, ctx, warnings, thermal, payload_bytes })
}

#[allow(clippy::too_many_lines)]
pub fn run_acquisition(
    source: &Source,
    opts: &AcquisitionOptions,
) -> Result<AcquisitionReport, ControllerError> {
    if opts.frames == 0 {
        return Err(config("frames must be positive"));
    }
    if opts.project.is_empty() {
        return Err(config("project must be nonempty"));
    }
    let mut backend = connect_source(source)?;
    let caps_list = backend.discover()?;
    let caps = pick_camera(&caps_list)?.clone();
    let now_ns = now_unix_ns()?;
    let PreparedRun { outcome, stream, mut store, disk: worker, session_dir, session_id, stamp: _stamp, ctx, mut warnings, .. } =
        prepare_run(&mut *backend, &caps, opts, now_ns)?;
    let mut runner = SequenceRunner::new(opts.frames, RetryPolicy::default());
    runner.start()?;
    let run_result = run_frames(
        &mut *backend,
        &mut runner,
        &worker,
        &mut store,
        &outcome,
        &stream,
        &ctx.dir_name,
        ctx.kind_char,
        &ctx.project,
        &ctx.stamp,
        &ctx.ext,
        opts,
        now_ns,
    );
    // Always join the worker; receipts were awaited per frame, shutdown only drains.
    worker.shutdown()?;
    let frame_sha256 = match run_result {
        Ok(hashes) => {
            debug_assert_eq!(runner.state(), SequenceState::Completed);
            hashes
        }
        Err(e) => {
            let mut manifest = store.manifest().clone();
            manifest.errors.push(e.to_string());
            let _ = store.save(manifest);
            return Err(e);
        }
    };

    let preview_mean_luma = match backend.preview() {
        Ok(preview) => preview_to_rgb8(&preview).map(|rgb| rgb.mean_luma()).ok(),
        Err(e) => {
            warnings.push(format!("closing preview unavailable: {e}"));
            None
        }
    };
    let _ = backend.close();

    Ok(AcquisitionReport {
        session_dir,
        session_id,
        frames_committed: runner.progress().done,
        frame_sha256,
        preview_mean_luma,
        warnings,
        origin: caps.origin.unwrap_or(Origin::Synthetic),
        identity: caps.identity.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn run_frames(
    backend: &mut dyn CameraBackend,
    runner: &mut SequenceRunner,
    worker: &DiskWorker,
    store: &mut SessionStore,
    outcome: &deepsky_camera::model::ConfigurationOutcome,
    stream: &deepsky_camera::model::StreamConfiguration,
    dir_name: &str,
    kind_char: char,
    project: &str,
    stamp: &str,
    ext: &str,
    opts: &AcquisitionOptions,
    run_unix_ns: u64,
) -> Result<Vec<String>, ControllerError> {
    let mut hashes = Vec::new();
    let mut since_thermal = 0u32;
    let ctx = FrameCtx {
        dir_name: dir_name.to_string(),
        kind_char,
        project: project.to_string(),
        stamp: stamp.to_string(),
        ext: ext.to_string(),
    };
    while let Some(index) = runner.next_frame()? {
        // Per-frame receive clock; falls back to run start when the OS clock fails.
        let at_unix_ns = now_unix_ns().unwrap_or(run_unix_ns);
        if since_thermal >= 10 {
            let status = backend.thermal()?;
            let mut warnings = Vec::new();
            if let Err(e) = check_thermal(&status, &mut warnings) {
                let _ = runner.stop();
                return Err(e);
            }
            since_thermal = 0;
        }
        since_thermal += 1;
        match capture_and_commit(backend, worker, outcome, stream, &ctx, index, at_unix_ns) {
            Ok(frame_record) => {
                hashes.push(frame_record.sha256.clone());
                store.record_frame(frame_record)?;
                runner.frame_committed(index)?;
            }
            // Storage failure on an already-captured frame: fatal, never skip.
            Err(e @ (ControllerError::Io(_) | ControllerError::Sequencer(_))) => {
                let _ = runner.frame_failed(index, false);
                return Err(e);
            }
            Err(ControllerError::Camera(e)) => {
                // Timeouts and I/O errors may clear; anything else fails the run.
                // The retry budget bounds the attempts; recovery needs evidence.
                let retryable = matches!(e.code, ErrorCode::Timeout | ErrorCode::Io);
                match runner.frame_failed(index, retryable) {
                    Ok(()) => {
                        if runner.state() == SequenceState::Error {
                            return Err(ControllerError::Camera(e));
                        }
                    }
                    Err(retry) => return Err(ControllerError::Sequencer(retry)),
                }
                if retryable {
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
            Err(e) => {
                let _ = runner.frame_failed(index, false);
                return Err(e);
            }
        }
        if opts.delay_ns > 0 {
            std::thread::sleep(Duration::from_nanos(opts.delay_ns));
        }
    }
    Ok(hashes)
}

#[allow(clippy::too_many_arguments)]
/// Capture one frame and wait for its durable write receipt. The manifest
/// commit stays with the caller so runner law and manifest stay in lockstep.
pub fn capture_and_commit(
    backend: &mut dyn CameraBackend,
    worker: &DiskWorker,
    outcome: &deepsky_camera::model::ConfigurationOutcome,
    stream: &deepsky_camera::model::StreamConfiguration,
    ctx: &FrameCtx,
    index: u32,
    run_unix_ns: u64,
) -> Result<FrameRecord, ControllerError> {
    let captured = backend.capture()?;
    submit_frame(worker, outcome, stream, &captured.metadata, captured.payload, ctx, index, run_unix_ns)
}

/// Frame naming inputs, fixed at run start.
pub struct FrameCtx {
    pub dir_name: String,
    pub kind_char: char,
    pub project: String,
    pub stamp: String,
    pub ext: String,
}

#[allow(clippy::too_many_arguments)]
fn submit_frame(
    worker: &DiskWorker,
    outcome: &deepsky_camera::model::ConfigurationOutcome,
    stream: &deepsky_camera::model::StreamConfiguration,
    reported_meta: &CaptureMetadata,
    payload: Vec<u8>,
    ctx: &FrameCtx,
    index: u32,
    run_unix_ns: u64,
) -> Result<FrameRecord, ControllerError> {
    if stream.format == PixelFormat::Raw16Le {
        let expected = (u64::from(stream.width) * u64::from(stream.height) * 2) as usize;
        if payload.len() != expected {
            return Err(config(format!(
                "RAW payload {} bytes does not match {}x{} stream",
                payload.len(),
                stream.width,
                stream.height
            )));
        }
    }
    let requested = &outcome.requested.settings;
    let reported = &reported_meta.reported;
    let focus_map = |f: &Option<FocusRequest>| match f {
        Some(FocusRequest::Manual { millidiopters, .. }) => (Some(f64::from(*millidiopters as u32) / 1000.0), Some("manual".to_string())),
        Some(FocusRequest::Auto { mode, .. }) => (None, Some(format!("auto:{mode}"))),
        None => (None, None),
    };
    let (focus_req, focus_mode) = focus_map(&requested.focus);
    let (focus_rep, _) = focus_map(&reported.focus);
    let wb_map = |w: &Option<WhiteBalanceRequest>| match w {
        Some(WhiteBalanceRequest::Mode(m)) => (Some(m.clone()), None, None),
        Some(WhiteBalanceRequest::Temperature { kelvin: _, .. }) => (Some("temperature".to_string()), None, None),
        Some(WhiteBalanceRequest::Manual { gains_x1000, transform_millionths }) => (
            Some("manual".to_string()),
            Some([gains_x1000[0] as f64 / 1000.0, gains_x1000[1] as f64 / 1000.0, gains_x1000[2] as f64 / 1000.0, gains_x1000[3] as f64 / 1000.0]),
            Some([
                transform_millionths[0] as f64 / 1e6, transform_millionths[1] as f64 / 1e6, transform_millionths[2] as f64 / 1e6,
                transform_millionths[3] as f64 / 1e6, transform_millionths[4] as f64 / 1e6, transform_millionths[5] as f64 / 1e6,
                transform_millionths[6] as f64 / 1e6, transform_millionths[7] as f64 / 1e6, transform_millionths[8] as f64 / 1e6,
            ]),
        ),
        None => (None, None, None),
    };
    let (wb_mode, wb_gains, wb_transform) = wb_map(&reported.white_balance);
    let layout = reported_meta.raw_layout.clone();
    let metadata = FrameMetadata {
        frame_id: format!("{index:06}"),
        camera_id: outcome.requested.selection.camera_id.clone(),
        exposure_requested_ns: requested.exposure_ns.unwrap_or(0),
        exposure_reported_ns: reported.exposure_ns,
        sensitivity_requested: requested.sensitivity.unwrap_or(0).try_into().map_err(|_| config("sensitivity overflow"))?,
        sensitivity_reported: reported.sensitivity.map(|v| v.try_into().map_err(|_| config("reported sensitivity overflow"))).transpose()?,
        timestamp_ns: reported_meta.start_ns,
        physical_camera_id: outcome.requested.selection.physical_id.clone().or(reported_meta.active_physical_id.clone()),
        request_id: None, // captures carry no per-frame request id on this transport
        sequence_id: None,
        frame_number: Some(u64::from(index)),
        received_at_unix_ns: Some(run_unix_ns),
        clock_offset_ns: None, // device clock unknown until the Kotlin side reports it
        clock_uncertainty_ns: None,
        width: Some(stream.width),
        height: Some(stream.height),
        pixel_format: Some(format!("{:?}", stream.format)),
        row_stride_bytes: layout.as_ref().map(|l| l.row_stride_bytes),
        bits_per_sample: layout.as_ref().map(|l| l.bit_depth),
        byte_order: layout.as_ref().map(|_| "little".to_string()),
        cfa_pattern: layout.as_ref().and_then(|l| l.cfa.clone()),
        black_levels: layout.as_ref().and_then(|l| l.black_levels.map(|b| [b[0] as f64, b[1] as f64, b[2] as f64, b[3] as f64])),
        white_level: layout.as_ref().and_then(|l| l.white_level.map(u32::from)),
        active_area: None,
        color_matrix: None, // never invented: stays absent without device calibration
        calibration_illuminant: None,
        frame_duration_requested_ns: requested.frame_duration_ns,
        frame_duration_reported_ns: reported.frame_duration_ns,
        focus_requested_diopters: focus_req,
        focus_reported_diopters: focus_rep,
        focus_mode,
        white_balance_mode: wb_mode,
        white_balance_gains: wb_gains,
        white_balance_transform: wb_transform,
        crop_region: reported.crop.map(|c| [c.x, c.y, c.width, c.height]),
        zoom_requested: requested.zoom_x1000.map(|z| z as f64 / 1000.0),
        zoom_reported: reported.zoom_x1000.map(|z| z as f64 / 1000.0),
        ois_mode: reported.ois.clone(),
        processing_modes: reported.processing.clone(),
        sensor_temperature_c: None,
        device_temperature_c: None,
        orientation_degrees: None,
        calibration_kind: None,
        app_version: Some(APP_VERSION.into()),
        protocol_version: Some(PROTOCOL_VERSION.to_string()),
        android_version: None,
        config_version: Some(CONFIG_VERSION),
        warnings: vec![],
        extra: [
            ("origin".to_string(), serde_json::json!(format!("{:?}", reported_meta.origin))),
            ("identity".to_string(), serde_json::json!(reported_meta.identity)),
            ("timestamp_domain".to_string(), serde_json::json!(reported_meta.timestamp_domain)),
        ]
        .into_iter()
        .collect(),
    };
    let filename = frame_filename(&ctx.project, &ctx.stamp, ctx.kind_char, index, &ctx.ext);
    let relative_path: PathBuf = Path::new(&ctx.dir_name).join(&filename);
    let receipt = worker
        .submit(WriteJob { relative_path, data: payload, metadata })
        .map_err(|e| ControllerError::Io(io::Error::new(e.kind, e.message)))?;
    receipt.wait().map_err(ControllerError::from)
}

/// Encode packed RGB8 as PNG for UI preview transport (never RAW bytes).
pub fn encode_png_rgb8(rgb: &[u8], width: u32, height: u32) -> io::Result<Vec<u8>> {
    if rgb.len() != width as usize * height as usize * 3 || width == 0 || height == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "RGB dimensions do not match byte count"));
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(rgb)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Source;

    #[test]
    fn utc_stamp_known_value() {
        assert_eq!(unix_to_utc_stamp(0), "1970-01-01T000000Z");
        assert_eq!(unix_to_utc_stamp(1_000_000_000), "2001-09-09T014640Z");
    }

    #[test]
    fn rejects_empty_plan() {
        let opts = AcquisitionOptions { frames: 0, ..Default::default() };
        assert!(matches!(run_acquisition(&Source::Simulator { realtime: false }, &opts), Err(ControllerError::Config(_))));
    }

    #[test]
    fn single_frame_end_to_end() {
        let dir = std::env::temp_dir().join(format!("deepsky-test-{}-single", std::process::id()));
        let opts = AcquisitionOptions { out_dir: dir, ..Default::default() };
        let report = run_acquisition(&Source::Simulator { realtime: false }, &opts).unwrap();
        assert_eq!(report.frames_committed, 1);
        assert_eq!(report.frame_sha256.len(), 1);
        assert!(report.session_dir.join("session.json").exists());
        let entries: Vec<_> = std::fs::read_dir(report.session_dir.join("test")).unwrap().collect();
        assert_eq!(entries.len(), 2); // .raw16 payload + .json sidecar
        std::fs::remove_dir_all(&report.session_dir).unwrap();
    }

    #[test]
    fn three_frame_run_with_preview() {
        let dir = std::env::temp_dir().join(format!("deepsky-test-{}-three", std::process::id()));
        let opts = AcquisitionOptions { out_dir: dir, frames: 3, ..Default::default() };
        let report = run_acquisition(&Source::Simulator { realtime: false }, &opts).unwrap();
        assert_eq!(report.frames_committed, 3);
        assert!(report.preview_mean_luma.is_some());
        let store = SessionStore::open(&report.session_dir).unwrap();
        assert_eq!(store.manifest().frames_completed, 3);
        assert!(store.scan().unwrap().can_resume());
        std::fs::remove_dir_all(&report.session_dir).unwrap();
    }
}
