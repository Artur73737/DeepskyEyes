//! Desktop worker thread: handles every `UiAction` with real backend
//! operations and pushes authoritative `UiSnapshot`s. The UI owns
//! presentation only; this worker owns the camera, the run and the files.
//!
//! Captures expose on a dedicated thread so progress snapshots keep flowing;
//! the worker polls, commits and reports. No mock data anywhere here.
use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
    time::{Duration, Instant},
};

use deepsky_camera::{
    backend::CameraBackend,
    model::{CameraCapabilities, CapturedFrame, ErrorCode, ThermalStatus, ValidationPolicy},
};
use deepsky_preview::{decoder::preview_to_rgb8, histogram::Histogram};
use deepsky_sequencer::{recovery::RetryPolicy, runner::SequenceRunner, state_machine::SequenceState};
use deepsky_session::{calibration::SessionKind, store::SessionStore};
use deepsky_ui::{CameraChoice, FrameType, PreviewImage, SequenceStatus, SessionSummary, SourceKind, UiAction, UiSnapshot};

use crate::{
    controller::{
        build_request, capture_and_commit, check_thermal, connect_source, encode_png_rgb8, now_unix_ns,
        pick_camera, prepare_run, submit_frame, AcquisitionOptions, ControllerError, PreparedRun,
        APP_VERSION,
    },
    source::Source,
};

/// Settle delay between sequence frames (README example: 1 s).
const SETTLE_DELAY_NS: u64 = 1_000_000_000;
/// Re-read device heat at most every N committed frames.
const THERMAL_EVERY_FRAMES: u32 = 10;

pub fn source_from_env() -> Source {
    match std::env::var("DEEPSKY_SOURCE").unwrap_or_default().as_str() {
        "" => Source::Adb(None),
        "sim" => Source::Simulator { realtime: false },
        "sim:realtime" => Source::Simulator { realtime: true },
        s if s.starts_with("tcp:") => Source::Tcp(s["tcp:".len()..].to_string()),
        "adb" => Source::Adb(None),
        s if s.starts_with("adb:") => Source::Adb(Some(s["adb:".len()..].to_string())),
        other => {
            eprintln!("warning: invalid DEEPSKY_SOURCE '{other}', using physical ADB camera; no synthetic fallback");
            Source::Adb(None)
        }
    }
}

fn frame_kind_to_session(kind: FrameType) -> SessionKind {
    match kind {
        FrameType::Light => SessionKind::Light,
        FrameType::Dark => SessionKind::Dark,
        FrameType::Flat => SessionKind::Flat,
        FrameType::Bias => SessionKind::Bias,
    }
}

struct ActiveRun {
    runner: SequenceRunner,
    prepared: PreparedRun,
    last_step: Instant,
    since_thermal: u32,
    /// Frame currently exposing on the capture thread, if any.
    capture: Option<CaptureInFlight>,
    /// Stop requested mid-exposure: honored as soon as the frame lands.
    pending_stop: bool,
}

/// A frame exposing off the worker thread. The thread owns the backend link;
/// the worker keeps stepping, polling and reporting live progress meanwhile.
struct CaptureInFlight {
    rx: Receiver<CaptureThreadOut>,
    started: Instant,
    exposure_s: f32,
    index: u32,
}

type CaptureThreadOut = (
    Box<dyn CameraBackend + Send>,
    Result<CapturedFrame, deepsky_camera::model::CameraError>,
);

type PreviewThreadOut = (
    Box<dyn CameraBackend + Send>,
    Result<deepsky_camera::model::PreviewFrame, deepsky_camera::model::CameraError>,
);

struct Worker {
    backend: Option<Box<dyn CameraBackend + Send>>,
    preview_in_flight: Option<Receiver<PreviewThreadOut>>,
    controls_changed: Option<Instant>,
    preview_started: Option<Instant>,
    preview_exposure_s: f32,
    source: SourceKind,
    /// Camera id currently open on the backend, if any. Single source of
    /// truth for open-state: prevents double-open across select/run paths.
    open_camera_id: Option<String>,
    caps: Vec<CameraCapabilities>,
    camera_id: String,
    outcome: Option<deepsky_camera::model::ConfigurationOutcome>,
    // Pending controls (mirrors the UI).
    exposure_ns: u64,
    sensitivity: u32,
    focus_mdiopt: u64,
    focus_locked: bool,
    wb_kelvin: u32,
    /// Pending announced WB preset; empty means "device default chain".
    wb_preset: String,
    zoom_x1000: Option<u64>,
    stream_override: Option<deepsky_camera::model::StreamConfiguration>,
    raw: bool,
    frames_total: u32,
    frame_kind: FrameType,
    project: String,
    destination: PathBuf,
    auto_save: bool,
    run: Option<ActiveRun>,
    message: String,
    device_label: String,
    frame_progress: Option<f32>,
    preview: Option<PreviewImage>,
    histogram: Vec<u32>,
    preview_mean: Option<f64>,
    preview_failures: u64,
    preview_revision: u64,
    /// Consecutive live-preview failures, used for bounded retry backoff.
    live_backoff: u32,
    last_sha: Option<String>,
    thermal: Option<ThermalStatus>,
    last_bytes: Option<(u64, u64, Instant)>,
    rx_mbps: f64,
    tx_mbps: f64,
    sessions_cache: Vec<SessionSummary>,
    sessions_checked: Option<Instant>,
}

impl Worker {
    fn new() -> Self {
        Self {
            backend: None,
            preview_in_flight: None,
            controls_changed: None,
            preview_started: None,
            preview_exposure_s: 0.0,
            source: SourceKind::Phone,
            open_camera_id: None,
            caps: vec![],
            camera_id: String::new(),
            outcome: None,
            exposure_ns: 15_000_000_000,
            sensitivity: 800,
            focus_mdiopt: 0,
            focus_locked: true,
            wb_kelvin: 5_000,
            wb_preset: String::new(),
            zoom_x1000: None,
            stream_override: None,
            raw: true,
            frames_total: 300,
            frame_kind: FrameType::Light,
            project: "M42".into(),
            destination: crate::controller::default_capture_dir(),
            auto_save: true,
            run: None,
            message: String::new(),
            device_label: "No device connected".into(),
            frame_progress: None,
            preview: None,
            histogram: vec![],
            preview_mean: None,
            preview_failures: 0,
            preview_revision: 0,
            live_backoff: 0,
            last_sha: None,
            thermal: None,
            last_bytes: None,
            rx_mbps: 0.0,
            tx_mbps: 0.0,
            sessions_cache: Vec::new(),
            sessions_checked: None,
        }
    }

    fn open_caps(&self) -> Option<&CameraCapabilities> {
        self.caps.iter().find(|c| c.camera_id == self.camera_id)
    }

    fn fail(&mut self, message: String) {
        self.message = message;
    }

    fn handle(&mut self, action: UiAction) {
        if local_control(&action) && !matches!(action,
            UiAction::SetFrameCount(_) | UiAction::SetFrameType(_) | UiAction::SetAutoSave(_)) {
            self.controls_changed = Some(Instant::now());
        }
        match action {
            UiAction::Connect => self.connect(),
            UiAction::Disconnect => self.disconnect(),
            UiAction::SetSource(source) => self.set_source(source),
            UiAction::SelectCamera(id) => self.select_camera(id),
            UiAction::SetExposure(ns) => self.set_exposure(ns),
            UiAction::SetIso(iso) => self.set_iso(iso),
            UiAction::SetFocus(diopters) => self.set_focus(diopters),
            UiAction::AutofocusCenter => {
                if self.run.is_some() { self.fail("stop the sequence before autofocus".into()); }
                else if let Some(backend) = self.backend.as_mut() {
                    match backend.autofocus_center() {
                        Ok(distance) => {
                            self.focus_mdiopt = distance;
                            self.focus_locked = true;
                            self.controls_changed = Some(Instant::now() - Duration::from_millis(150));
                            self.message = format!("Center AF locked at {:.3} D", distance as f64 / 1000.0);
                        }
                        Err(error) => self.fail(format!("autofocus: {error}")),
                    }
                } else { self.fail("connect and open a camera first".into()); }
            }
            UiAction::SetWhiteBalance(kelvin) => self.set_white_balance(kelvin),
            UiAction::SetWbPreset(preset) => self.set_wb_preset(preset),
            UiAction::SetZoom(zoom) => self.set_zoom(zoom),
            UiAction::SetRaw(raw) => {
                self.raw = raw;
                self.stream_override = None;
                self.message.clear();
            }
            UiAction::SetResolution(w, h) => self.set_resolution(w, h),
            UiAction::SetLocked(locked) => {
                self.focus_locked = locked;
                self.message.clear();
            }
            UiAction::SetFrameCount(n) => {
                if n == 0 {
                    self.fail("frame count must be positive".into());
                } else {
                    self.frames_total = n;
                    self.message.clear();
                }
            }
            UiAction::StartSequence => self.start_sequence(),
            UiAction::CaptureOne => {
                if self.auto_save {
                    let count = self.frames_total;
                    self.frames_total = 1;
                    self.start_sequence();
                    self.frames_total = count;
                } else { self.capture_one(); }
            }
            UiAction::PauseSequence => self.pause_resume_stop("pause"),
            UiAction::ResumeSequence => self.pause_resume_stop("resume"),
            UiAction::StopSequence => self.pause_resume_stop("stop"),
            UiAction::SetFrameType(kind) => {
                self.frame_kind = kind;
                self.message.clear();
            }
            UiAction::RefreshSessions => {
                self.sessions_checked = None;
                let count = self.sessions().len();
                self.message = format!("{} session(s) in {}", count, self.destination.display());
            }
            UiAction::OpenSession(id) => self.open_session(id),
            UiAction::RefreshDiagnostics => self.refresh_diagnostics(),
            UiAction::ChooseDestination => {
                self.fail("folder picker is UI-local and never reaches the worker".into());
            }
            UiAction::SetDestination(path) => {
                let dir = PathBuf::from(&path);
                if dir.is_dir() {
                    self.destination = dir;
                    self.sessions_checked = None;
                    self.message.clear();
                } else {
                    self.fail(format!("destination is not a directory: {path}"));
                }
            }
            UiAction::SetAutoSave(auto) => {
                self.auto_save = auto;
                self.message.clear();
            }
            UiAction::Shutdown => {}
        }
    }

    fn active_source(&self) -> Source {
        // Explicit env override wins (scripting/CI); otherwise the UI choice.
        // Phone = Pixel over ADB, the default. Simulator is explicit opt-in.
        if std::env::var("DEEPSKY_SOURCE").unwrap_or_default().is_empty() {
            match self.source {
                SourceKind::Phone => Source::Adb(None),
                SourceKind::Simulator => Source::Simulator { realtime: false },
            }
        } else {
            source_from_env()
        }
    }

    fn set_source(&mut self, source: SourceKind) {
        if self.backend.is_some() || self.run.is_some() {
            self.fail("disconnect first, then switch source".into());
            return;
        }
        self.source = source;
        self.message.clear();
    }

    fn connect(&mut self) {
        if self.backend.is_some() || self.run.is_some() {
            self.fail("already connected".into());
            return;
        }
        self.connect_with(self.active_source());
    }

    fn connect_with(&mut self, source: Source) {
        match connect_source(&source) {
            Ok(mut backend) => match backend.discover() {
                Ok(caps) => {
                    let label = match pick_camera(&caps) {
                        Ok(picked) => {
                            self.camera_id = picked.camera_id.clone();
                            format!("{} ({:?})", picked.identity, picked.origin)
                        }
                        Err(e) => {
                            self.camera_id.clear();
                            format!("connected, but discovery is empty: {e}")
                        }
                    };
                    self.caps = caps;
                    self.backend = Some(backend);
                    self.device_label = self
                        .backend
                        .as_ref()
                        .map(|b| b.id().to_string())
                        .unwrap_or_else(|| "connected".into());
                    self.thermal = self.backend.as_mut().and_then(|b| b.thermal().ok());
                    self.message = format!("connected: {label}");
                    if !self.camera_id.is_empty() {
                        self.select_camera(self.camera_id.clone());
                    }
                }
                Err(e) => self.fail(format!("discover: {e}")),
            },
            Err(e) => self.fail(format!("connect: {e}")),
        }
    }

    fn disconnect(&mut self) {
        if let Some(run) = self.run.take() {
            let _ = self.abort_run(run, "disconnected by user");
        }
        if let Some(mut backend) = self.backend.take() {
            let _ = backend.close();
        }
        self.caps.clear();
        self.camera_id.clear();
        self.open_camera_id = None;
        self.outcome = None;
        self.device_label = "No device connected".into();
        self.preview = None;
        self.histogram.clear();
        self.preview_mean = None;
        self.frame_progress = None;
        self.message = "disconnected".into();
    }

    /// Close the camera if the worker opened one. Run paths call this before
    /// prepare_run (which always opens fresh), so double-open is impossible.
    fn ensure_closed(&mut self) -> Result<(), String> {
        if self.open_camera_id.is_some() {
            let backend = self.backend.as_mut().ok_or_else(|| "not connected".to_string())?;
            backend.close().map_err(|e| format!("close: {e}"))?;
            self.open_camera_id = None;
            self.outcome = None;
        }
        Ok(())
    }

    fn select_camera(&mut self, id: String) {
        if self.run.is_some() {
            self.fail("stop the sequence before switching camera".into());
            return;
        }
        if self.open_camera_id.as_deref() == Some(id.as_str()) {
            self.camera_id = id.clone();
            self.message = format!("camera {id} already open");
            self.refresh_preview();
            return;
        }
        if !self.caps.iter().any(|c| c.camera_id == id) {
            self.fail(format!("camera '{id}' not announced"));
            return;
        }
        if let Err(e) = self.ensure_closed() {
            self.fail(e);
            return;
        }
        let caps = self.caps.iter().find(|c| c.camera_id == id).unwrap().clone();
        if self.backend.is_none() {
            self.fail("not connected".into());
            return;
        }
        let opts = self.options(1);
        let backend = self.backend.as_mut().expect("connected");
        let selection = deepsky_camera::model::CameraSelection { camera_id: id.clone(), physical_id: None };
        if let Err(e) = backend.open(&selection) {
            self.open_camera_id = None;
            self.fail(format!("open {id}: {e}"));
            return;
        }
        self.open_camera_id = Some(id.clone());
        // Configure immediately with the pending settings so live preview
        // starts now instead of only after the first capture.
        match build_request(&caps, &opts.spec())
            .map_err(|e| e.to_string())
            .and_then(|request| backend.configure(&request, ValidationPolicy::Reject).map_err(|e| e.to_string()))
        {
            Ok(outcome) => {
                self.camera_id = id.clone();
                self.outcome = Some(outcome);
                self.live_backoff = 0;
                self.message = format!("camera {id} open, live preview on");
                self.refresh_preview();
            }
            Err(e) => {
                let _ = backend.close();
                self.open_camera_id = None;
                self.fail(format!("configure {id}: {e}"));
            }
        }
    }

    fn set_exposure(&mut self, ns: u64) {
        match self.open_caps().and_then(|c| c.exposure_ns) {
            Some(range) if ns >= range.min && ns <= range.max => {
                self.exposure_ns = ns;
                self.message.clear();
            }
            Some(range) => self.fail(format!("exposure {ns} ns outside announced {}..={}", range.min, range.max)),
            None => self.fail("exposure range not announced".into()),
        }
    }

    fn set_iso(&mut self, iso: u32) {
        match self.open_caps().and_then(|c| c.sensitivity) {
            Some(range) if u64::from(iso) >= range.min && u64::from(iso) <= range.max => {
                self.sensitivity = iso;
                self.message.clear();
            }
            Some(range) => self.fail(format!("ISO {iso} outside announced {}..={}", range.min, range.max)),
            None => self.fail("sensitivity range not announced".into()),
        }
    }

    fn set_focus(&mut self, diopters: f32) {
        if !(0.0..=1000.0).contains(&diopters) {
            self.fail(format!("focus {diopters} diopters out of plausible range"));
            return;
        }
        let mdiopt = (diopters * 1000.0).round() as u64;
        match self.open_caps().and_then(|c| c.focus_millidiopters) {
            Some(range) if mdiopt >= range.min && mdiopt <= range.max => {
                self.focus_mdiopt = mdiopt;
                self.message.clear();
            }
            Some(range) => self.fail(format!("focus {diopters} outside announced {}..={} mdiopt", range.min, range.max)),
            None => self.fail("focus range not announced".into()),
        }
    }

    fn set_white_balance(&mut self, kelvin: u32) {
        let supported = self.open_caps().is_some_and(|c| {
            c.wb_modes.iter().any(|m| m == "temperature")
                && c.wb_kelvin.is_some_and(|r| u64::from(kelvin) >= r.min && u64::from(kelvin) <= r.max)
        });
        if supported {
            self.wb_kelvin = kelvin;
            self.message.clear();
        } else {
            self.fail(format!("white balance {kelvin} K not announced"));
        }
    }

    fn set_wb_preset(&mut self, preset: String) {
        let announced = self.open_caps().is_some_and(|c| {
            preset != "temperature"
                && preset != "manual"
                && c.wb_modes.iter().any(|m| m == &preset)
        });
        if announced {
            self.wb_preset = preset;
            self.message.clear();
        } else {
            self.fail(format!("white-balance preset '{preset}' not announced"));
        }
    }

    fn set_zoom(&mut self, zoom: f32) {
        if !zoom.is_finite() || zoom <= 0.0 {
            self.fail(format!("zoom {zoom}x must be finite and positive"));
            return;
        }
        let x1000 = (zoom * 1000.0).round() as u64;
        let supported = self.open_caps().is_some_and(|c| {
            c.zoom_x1000.is_some_and(|r| x1000 >= r.min && x1000 <= r.max)
        });
        if supported {
            self.zoom_x1000 = Some(x1000);
            self.message.clear();
        } else {
            self.fail(format!("zoom {zoom}x outside announced range"));
        }
    }

    fn set_resolution(&mut self, w: u32, h: u32) {
        let found = self.open_caps().and_then(|c| {
            c.streams
                .iter()
                .filter(|s| {
                    if self.raw {
                        matches!(s.format, deepsky_camera::model::PixelFormat::Raw16Le | deepsky_camera::model::PixelFormat::Dng)
                    } else {
                        !matches!(s.format, deepsky_camera::model::PixelFormat::Raw16Le | deepsky_camera::model::PixelFormat::Dng)
                    }
                })
                .find(|s| s.width == w && s.height == h)
                .cloned()
        });
        match found {
            Some(stream) => {
                self.stream_override = Some(stream);
                self.message.clear();
            }
            None => self.fail(format!("resolution {w}x{h} not announced for the current format")),
        }
    }

    fn options(&self, frames: u32) -> AcquisitionOptions {
        AcquisitionOptions {
            project: self.project.clone(),
            out_dir: self.destination.clone(),
            calibration: frame_kind_to_session(self.frame_kind),
            frames,
            exposure_ns: self.exposure_ns,
            sensitivity: self.sensitivity,
            focus_millidiopters: self.focus_mdiopt,
            wb_kelvin: u64::from(self.wb_kelvin),
            wb_preset: if self.wb_preset.is_empty() {
                None
            } else {
                Some(self.wb_preset.clone())
            },
            delay_ns: SETTLE_DELAY_NS,
            focus_locked: self.focus_locked,
            zoom_x1000: self.zoom_x1000,
            stream_override: self.stream_override.clone(),
            raw: self.raw,
        }
    }

    fn require_open(&self) -> Result<&CameraCapabilities, String> {
        if self.backend.is_none() {
            return Err("not connected".into());
        }
        self.open_caps().ok_or_else(|| "select a camera first".into())
    }

    fn start_sequence(&mut self) {
        if self.run.is_some() {
            self.fail("sequence already running".into());
            return;
        }
        if !self.auto_save {
            self.fail("enable auto-save: sequences are never run without storage".into());
            return;
        }
        if self.frames_total == 0 {
            self.fail("frame count must be positive".into());
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&self.destination) {
            self.fail(format!("destination: {e}"));
            return;
        }
        let caps = match self.require_open() {
            Ok(caps) => caps.clone(),
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        let now = match crate::controller::now_unix_ns() {
            Ok(now) => now,
            Err(e) => {
                self.fail(e.to_string());
                return;
            }
        };
        // Freeze the selected manual distance for the run, independently of
        // the UI edit lock. Do not change the requested focus distance.
        self.focus_locked = true;
        let opts = self.options(self.frames_total);
        if let Err(e) = self.ensure_closed() {
            self.fail(e);
            return;
        }
        let backend = self.backend.as_mut().expect("connected");
        let prepared = match prepare_run(&mut **backend, &caps, &opts, now) {
            Ok(prepared) => prepared,
            Err(e) => {
                let _ = backend.close();
                self.fail(e.to_string());
                return;
            }
        };
        let mut runner = SequenceRunner::new(self.frames_total, RetryPolicy::default());
        if let Err(e) = runner.start() {
            self.fail(format!("sequencer: {e}"));
            let _ = prepared.disk.shutdown();
            return;
        }
        self.outcome = Some(prepared.outcome.clone());
        self.open_camera_id = Some(caps.camera_id.clone());
        self.run = Some(ActiveRun {
            runner,
            prepared,
            last_step: Instant::now() - Duration::from_nanos(SETTLE_DELAY_NS),
            since_thermal: 0,
            capture: None,
            pending_stop: false,
        });
        self.message = format!("sequence started: {} frame(s)", self.frames_total);
    }

    fn capture_one(&mut self) {
        if self.run.is_some() {
            self.fail("stop the sequence before a single capture".into());
            return;
        }
        if !self.auto_save {
            // Preview-only path: no file is written and the message says so.
            self.refresh_preview();
            if self.preview.is_some() {
                self.message = "preview only: auto-save off, nothing written".into();
            }
            return;
        }
        let caps = match self.require_open() {
            Ok(caps) => caps.clone(),
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        if let Err(e) = std::fs::create_dir_all(&self.destination) {
            self.fail(format!("destination: {e}"));
            return;
        }
        let now = match now_unix_ns() {
            Ok(now) => now,
            Err(e) => {
                self.fail(e.to_string());
                return;
            }
        };
        let opts = self.options(1);
        if let Err(e) = self.ensure_closed() {
            self.fail(e);
            return;
        }
        let backend = self.backend.as_mut().expect("connected");
        let mut prepared = match prepare_run(&mut **backend, &caps, &opts, now) {
            Ok(prepared) => prepared,
            Err(e) => {
                self.fail(e.to_string());
                return;
            }
        };
        let mut runner = SequenceRunner::new(1, RetryPolicy::default());
        let result = (|| -> Result<String, ControllerError> {
            runner.start()?;
            let index = runner.next_frame()?.ok_or(ControllerError::Config("empty run".into()))?;
            let record = capture_and_commit(
                &mut **backend,
                &prepared.disk,
                &prepared.outcome,
                &prepared.stream,
                &prepared.ctx,
                index,
                now,
            )?;
            prepared.store.record_frame(record.clone())?;
            runner.frame_committed(index)?;
            Ok(record.sha256)
        })();
        let _ = prepared.disk.shutdown();
        match result {
            Ok(sha) => {
                self.outcome = Some(prepared.outcome.clone());
                self.message = format!("1 frame committed to {} (sha256 {sha})", prepared.session_dir.display());
                self.refresh_preview();
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    fn pause_resume_stop(&mut self, op: &str) {
        let Some(run) = self.run.as_mut() else {
            self.fail(format!("{op}: no active sequence"));
            return;
        };
        if op == "pause" && run.capture.is_some() {
            self.fail("frame exposing, wait for frame end".into());
            return;
        }
        let result = match op {
            "pause" => run.runner.pause().map(|_| "paused".to_string()),
            "resume" => run.runner.resume().map(|_| "resumed".to_string()),
            _ => {
                if run.capture.is_some() {
                    run.pending_stop = true;
                    Ok("stopping after current frame".to_string())
                } else {
                    run.runner.stop().map(|_| "stopped".to_string())
                }
            }
        };
        match result {
            Ok(message) => {
                if op == "stop" {
                    let run = self.run.take().expect("active");
                    let _ = self.abort_run(run, "stopped by user");
                    self.message = "sequence stopped".into();
                } else {
                    self.message = format!("sequence {message}");
                }
            }
            Err(e) => self.fail(format!("{op}: {e}")),
        }
    }

    /// Shut down an active run's disk worker and record why it ended.
    fn abort_run(&mut self, mut run: ActiveRun, reason: &str) -> Result<(), String> {
        let result = run.prepared.disk.shutdown().map_err(|e| e.to_string());
        let mut manifest = run.prepared.store.manifest().clone();
        manifest.errors.push(reason.to_string());
        run.prepared.store.save(manifest).map_err(|e| e.to_string())?;
        result
    }

    /// Advance a running sequence. Polls an exposing frame every tick (so live
    /// progress flows); starts the next frame once its settle delay elapsed.
    fn step(&mut self) {
        let capturing = self.run.as_ref().is_some_and(|run| run.capture.is_some());
        if !capturing {
            let due = self.run.as_ref().is_some_and(|run| {
                run.runner.state() == SequenceState::Running && run.last_step.elapsed() >= Duration::from_nanos(SETTLE_DELAY_NS)
            });
            if !due {
                return;
            }
        }
        let mut run = self.run.take().expect("step implies active");
        if run.capture.is_none() && run.pending_stop {
            let _ = run.runner.stop();
            let _ = self.abort_run(run, "stopped by user");
            self.outcome = None;
            self.message = "sequence stopped".into();
            self.frame_progress = None;
            return;
        }
        let outcome = self.step_frame(&mut run);
        match outcome {
            StepOutcome::Wait => {
                self.run = Some(run);
            }
            StepOutcome::Continue => {
                run.last_step = Instant::now();
                self.run = Some(run);
            }
            StepOutcome::Finished(message) => {
                let _ = run.prepared.disk.shutdown();
                self.outcome = Some(run.prepared.outcome.clone());
                self.message = message;
                self.frame_progress = None;
                // The idle loop resumes live view asynchronously.
            }
            StepOutcome::Failed(message) => {
                let _ = self.abort_run(run, &message);
                self.outcome = None;
                self.message = message;
                self.frame_progress = None;
            }
        }
    }

    fn step_frame(&mut self, run: &mut ActiveRun) -> StepOutcome {
        // A frame is exposing off-thread: poll it, report live progress.
        if run.capture.is_some() {
            return self.poll_capture(run);
        }
        if run.since_thermal >= THERMAL_EVERY_FRAMES {
            let status = self.backend.as_mut().expect("run implies connected").thermal();
            match status {
                Ok(status) => {
                    self.thermal = Some(status.clone());
                    let mut warnings = Vec::new();
                    if check_thermal(&status, &mut warnings).is_err() {
                        let _ = run.runner.stop();
                        return StepOutcome::Failed(format!("thermal critical during sequence: {}", status.severity));
                    }
                    run.since_thermal = 0;
                }
                Err(e) => {
                    let _ = run.runner.stop();
                    return StepOutcome::Failed(format!("thermal unreadable during sequence: {e}"));
                }
            }
        }
        let index = match run.runner.next_frame() {
            Ok(Some(index)) => index,
            Ok(None) => {
                let done = run.runner.progress().done;
                return StepOutcome::Finished(format!("sequence complete: {done} frame(s) in {}", run.prepared.session_dir.display()));
            }
            Err(e) => {
                let _ = run.runner.stop();
                return StepOutcome::Failed(format!("sequencer: {e}"));
            }
        };
        run.since_thermal += 1;
        // Expose off the worker thread so progress snapshots keep flowing.
        // The backend link moves into the thread and comes back with the frame.
        let backend_box = self.backend.take().expect("run implies connected");
        let exposure_s = run
            .prepared
            .outcome
            .applied
            .settings
            .exposure_ns
            .or(run.prepared.outcome.requested.settings.exposure_ns)
            .map(|ns| ns as f32 / 1e9)
            .unwrap_or(0.0);
        let (tx, rx) = std::sync::mpsc::channel();
        let spawn = std::thread::Builder::new()
            .name(format!("deepsky-capture-{index}"))
            .spawn(move || {
                let mut backend_box = backend_box;
                let result = backend_box.capture();
                let _ = tx.send((backend_box, result));
            });
        match spawn {
            Ok(_) => {
                run.capture = Some(CaptureInFlight { rx, started: Instant::now(), exposure_s, index });
                self.frame_progress = Some(0.0);
                StepOutcome::Wait
            }
            Err(e) => {
                // The backend moved into the unborn thread: the link is lost.
                let _ = run.runner.stop();
                StepOutcome::Failed(fatal(run, format!("cannot spawn capture thread: {e}")))
            }
        }
    }

    /// Poll an exposing frame: restore the link and commit on landing,
    /// otherwise report live exposure progress.
    fn poll_capture(&mut self, run: &mut ActiveRun) -> StepOutcome {
        let cap = run.capture.as_mut().expect("polling live capture");
        match cap.rx.try_recv() {
            Ok((backend_box, capture_result)) => {
                self.backend = Some(backend_box);
                let index = cap.index;
                run.capture = None;
                self.frame_progress = None;
                match capture_result {
                    Ok(captured) => {
                        let now = now_unix_ns().unwrap_or(0);
                        match submit_frame(
                            &run.prepared.disk,
                            &run.prepared.outcome,
                            &run.prepared.stream,
                            &captured.metadata,
                            captured.payload,
                            &run.prepared.ctx,
                            index,
                            now,
                        ) {
                            Ok(record) => {
                                self.last_sha = Some(record.sha256.clone());
                                match run.prepared.store.record_frame(record) {
                                    Ok(()) => match run.runner.frame_committed(index) {
                                        Ok(()) => {
                                            // Never take a second exposure just
                                            // for preview between scientific RAWs.
                                            StepOutcome::Continue
                                        }
                                        Err(e) => StepOutcome::Failed(fatal(run, format!("commit law violated: {e}"))),
                                    },
                                    Err(e) => StepOutcome::Failed(fatal(run, format!("manifest commit: {e}"))),
                                }
                            }
                            Err(e) => StepOutcome::Failed(fatal(run, format!("storage failed: {e}"))),
                        }
                    }
                    Err(e) => {
                        let retryable = matches!(e.code, ErrorCode::Timeout | ErrorCode::Io);
                        match run.runner.frame_failed(index, retryable) {
                            Ok(()) if run.runner.state() == SequenceState::Error => {
                                StepOutcome::Failed(fatal(run, format!("capture failed: {e}")))
                            }
                            Ok(()) => {
                                std::thread::sleep(Duration::from_millis(200));
                                StepOutcome::Continue
                            }
                            Err(retry) => StepOutcome::Failed(fatal(run, format!("retry budget exhausted: {retry}"))),
                        }
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                let elapsed = cap.started.elapsed().as_secs_f32();
                let progress = if cap.exposure_s > 0.0 {
                    (elapsed / cap.exposure_s).min(1.0)
                } else {
                    0.0
                };
                self.frame_progress = Some(progress);
                StepOutcome::Wait
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The capture thread died holding the link: it is gone.
                run.capture = None;
                self.backend = None;
                self.frame_progress = None;
                let _ = run.runner.stop();
                StepOutcome::Failed(fatal(run, "capture thread died; camera link lost".into()))
            }
        }
    }

    fn refresh_preview(&mut self) {
        let Some(backend) = self.backend.as_mut() else { return };
        let result = backend.preview();
        self.accept_preview(result);
    }

    fn start_preview(&mut self) {
        let Some(mut backend) = self.backend.take() else { return };
        self.preview_started = Some(Instant::now());
        self.preview_exposure_s = self.outcome.as_ref().and_then(|o| o.applied.settings.exposure_ns)
            .unwrap_or(self.exposure_ns) as f32 / 1e9;
        let (tx, rx) = std::sync::mpsc::channel();
        self.preview_in_flight = Some(rx);
        std::thread::spawn(move || {
            let result = backend.preview();
            let _ = tx.send((backend, result));
        });
    }

    fn apply_live_controls(&mut self) {
        if !self.controls_changed.is_some_and(|at| at.elapsed() >= Duration::from_millis(150)) {
            return;
        }
        self.controls_changed = None;
        let Some(caps) = self.open_caps().cloned() else { return };
        let request = build_request(&caps, &self.options(1).spec());
        let Some(backend) = self.backend.as_mut() else { return };
        match request.map_err(|e| e.to_string()).and_then(|request|
            backend.configure(&request, ValidationPolicy::Reject).map_err(|e| e.to_string())) {
            Ok(outcome) => self.outcome = Some(outcome),
            Err(error) => self.fail(format!("live controls: {error}")),
        }
    }

    fn poll_preview(&mut self) -> bool {
        let Some(rx) = &self.preview_in_flight else { return false };
        match rx.try_recv() {
            Ok((backend, result)) => {
                self.preview_in_flight = None;
                self.preview_started = None;
                self.backend = Some(backend);
                self.live_backoff = if result.is_ok() { 0 } else { self.live_backoff.saturating_add(1) };
                self.accept_preview(result);
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.preview_in_flight = None;
                self.preview_started = None;
                self.open_camera_id = None;
                self.preview = None;
                self.fail("preview thread terminated; reconnect the camera".into());
                true
            }
        }
    }

    fn accept_preview(&mut self, result: Result<deepsky_camera::model::PreviewFrame, deepsky_camera::model::CameraError>) {
        match result {
            Ok(frame) => match preview_to_rgb8(&frame) {
                Ok(rgb) => match encode_png_rgb8(&rgb.rgb, rgb.width, rgb.height) {
                    Ok(png) => {
                        self.preview_revision += 1;
                        self.preview = Some(PreviewImage { revision: self.preview_revision, bytes: Arc::from(png) });
                        let histogram = Histogram::from_rgb8(&rgb.rgb);
                        self.histogram = histogram.r.iter().chain(&histogram.g).chain(&histogram.b).copied().collect();
                        self.preview_mean = Some(rgb.mean_luma());
                    }
                    Err(e) => {
                        self.preview_failures += 1;
                        self.message = format!("preview encode: {e}");
                    }
                },
                Err(e) => {
                    self.preview_failures += 1;
                    self.message = format!("preview decode: {e}");
                }
            },
            Err(e) => {
                self.preview_failures += 1;
                self.message = format!("preview: {e}");
            }
        }
    }

    fn refresh_diagnostics(&mut self) {
        let discovered = self.backend.as_mut().map(|backend| backend.discover());
        match discovered {
            Some(Ok(caps)) => {
                self.caps = caps;
                self.message = "diagnostics refreshed".into();
            }
            Some(Err(e)) => self.fail(format!("re-discover: {e}")),
            None => self.fail("not connected".into()),
        }
        if self.backend.is_some() {
            let thermal = self.backend.as_mut().map(|backend| backend.thermal());
            match thermal {
                Some(Ok(status)) => {
                    let mut warnings = Vec::new();
                    if check_thermal(&status, &mut warnings).is_err() {
                        self.thermal = Some(status.clone());
                        self.fail(format!("thermal critical: {}", status.severity));
                        return;
                    }
                    for warning in warnings {
                        self.message = warning;
                    }
                    self.thermal = Some(status);
                }
                Some(Err(e)) => self.fail(format!("thermal: {e}")),
                None => {}
            }
        }
        self.refresh_preview();
        self.update_mbps();
    }

    fn update_mbps(&mut self) {
        let stats = self.backend.as_ref().and_then(|b| b.transport_stats());
        let now = Instant::now();
        if let (Some((rx, tx)), Some((prx, ptx, at))) = (stats, self.last_bytes) {
            let dt = now.duration_since(at).as_secs_f64();
            if dt > 0.0 {
                self.rx_mbps = rx.saturating_sub(prx) as f64 * 8.0 / dt / 1e6;
                self.tx_mbps = tx.saturating_sub(ptx) as f64 * 8.0 / dt / 1e6;
            }
        }
        if let Some((rx, tx)) = stats {
            self.last_bytes = Some((rx, tx, now));
        }
    }

    fn sessions(&self) -> Vec<SessionSummary> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.destination) else { return out };
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.join("session.json").is_file() {
                continue;
            }
            if let Ok(store) = SessionStore::open(&dir) {
                let manifest = store.manifest();
                out.push(SessionSummary {
                    id: manifest.session_id.clone(),
                    name: manifest.project.clone(),
                    frames: manifest.frames_completed,
                    status: format!("{}/{} frames", manifest.frames_completed, manifest.frames_requested),
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    fn open_session(&mut self, id: String) {
        let found = std::fs::read_dir(&self.destination)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|dir| dir.join("session.json").is_file())
            .find_map(|dir| {
                SessionStore::open(&dir).ok().filter(|store| store.manifest().session_id == id).map(|store| (dir, store))
            });
        match found {
            Some((dir, store)) => {
                let manifest = store.manifest();
                match store.scan() {
                    Ok(report) => {
                        self.message = format!(
                            "{}: {} {}/{} frames, verified {}, missing {}, corrupt {}",
                            dir.display(),
                            manifest.project,
                            manifest.frames_completed,
                            manifest.frames_requested,
                            report.verified.len(),
                            report.missing.len(),
                            report.corrupt.len(),
                        );
                    }
                    Err(e) => self.fail(format!("session scan: {e}")),
                }
            }
            None => self.fail(format!("session '{id}' not found in {}", self.destination.display())),
        }
    }

    fn snapshot(&mut self) -> UiSnapshot {
        self.update_mbps();
        let caps = self.open_caps().cloned();
        let (run_state, done, total) = match &self.run {
            Some(run) => (run.runner.state(), run.runner.progress().done, run.runner.progress().total),
            None => (SequenceState::Idle, 0, self.frames_total),
        };
        let sequence = match run_state {
            SequenceState::Idle | SequenceState::Stopped => SequenceStatus::Idle,
            SequenceState::Running => SequenceStatus::Running,
            SequenceState::Paused => SequenceStatus::Paused,
            SequenceState::Completed => SequenceStatus::Completed,
            SequenceState::Error | SequenceState::FatalError => SequenceStatus::Failed,
            SequenceState::Validating | SequenceState::Preparing | SequenceState::Recovering | SequenceState::Completing => SequenceStatus::Running,
        };
        let mut snap = UiSnapshot::default();
        let capturing = self.run.as_ref().is_some_and(|run| run.capture.is_some());
        snap.connected = self.backend.is_some() || capturing || self.preview_in_flight.is_some();
        snap.device = self
            .backend
            .as_ref()
            .map(|b| b.id().to_string())
            .unwrap_or_else(|| self.device_label.clone());
        snap.camera_id = self.camera_id.clone();
        snap.cameras = self.caps.iter().map(|c| CameraChoice {
            id: c.camera_id.clone(),
            label: format!("{} ({})", c.camera_id, c.lens_role.clone().unwrap_or_else(|| "unknown lens".into())),
        }).collect();
        if let Some(caps) = &caps {
            snap.hardware_level = caps.hardware_level.clone().unwrap_or_else(|| "Unavailable".into());
            snap.sensor = format!("{} streams, {} preview", caps.streams.len(), caps.preview_streams.len());
            snap.resolutions = caps.streams.iter().map(|s| (s.width, s.height)).collect();
            snap.raw_supported = caps.raw == Some(true);
            snap.exposure_range_ns = caps.exposure_ns.map(|r| (r.min, r.max));
            snap.iso_range = caps.sensitivity.map(|r| (r.min as u32, r.max as u32));
            snap.focus_range = caps.focus_millidiopters.map(|r| (r.min as f32 / 1000.0, r.max as f32 / 1000.0));
            snap.manual_white_balance = caps.wb_modes.iter().any(|m| m == "manual" || m == "temperature");
            snap.zoom_range = caps.zoom_x1000.map(|r| (r.min as f32 / 1000.0, r.max as f32 / 1000.0));
        }
        if let Some(outcome) = &self.outcome {
            let applied = &outcome.applied.settings;
            snap.applied_exposure_ns = applied.exposure_ns;
            snap.applied_iso = applied.sensitivity.map(|v| v as u32);
            snap.resolution = applied.stream.as_ref().map(|s| (s.width, s.height)).unwrap_or((0, 0));
        }
        // Toggle shows the pending request (applies at next configure);
        // the applied stream stays visible in diagnostics/statistics.
        snap.raw_enabled = self.raw;
        snap.locked = self.focus_locked;
        snap.exposure_ns = self.exposure_ns;
        snap.iso = self.sensitivity;
        snap.focus_diopters = self.focus_mdiopt as f32 / 1000.0;
        snap.white_balance_kelvin = self.wb_kelvin;
        snap.wb_preset = self.wb_preset.clone();
        snap.wb_presets = self
            .open_caps()
            .map(|c| {
                c.wb_modes
                    .iter()
                    .filter(|m| *m != "temperature" && *m != "manual")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        snap.zoom = self.zoom_x1000.map(|z| z as f32 / 1000.0).unwrap_or(1.0);
        snap.frames_done = done;
        snap.frames_total = total;
        snap.sequence = sequence;
        snap.frame_progress = self.frame_progress;
        if let Some(started) = self.preview_started {
            snap.frame_progress = Some((started.elapsed().as_secs_f32() / self.preview_exposure_s.max(0.000001)).min(1.0));
            snap.frame_elapsed_s = Some(started.elapsed().as_secs_f32());
            snap.frame_exposure_s = Some(self.preview_exposure_s);
        } else if let Some(capture) = self.run.as_ref().and_then(|run| run.capture.as_ref()) {
            snap.frame_elapsed_s = Some(capture.started.elapsed().as_secs_f32());
            snap.frame_exposure_s = Some(capture.exposure_s);
        }
        snap.frame_type = self.frame_kind;
        snap.preview = self.preview.clone();
        snap.histogram = self.histogram.clone();
        snap.statistics = vec![
            ("identity".into(), snap.device.clone()),
            ("origin".into(), caps.as_ref().and_then(|c| c.origin.clone()).map(|o| format!("{o:?}")).unwrap_or_else(|| "none".into())),
            ("session dir".into(), self.run.as_ref().map(|r| r.prepared.session_dir.display().to_string()).unwrap_or_else(|| self.destination.display().to_string())),
            ("preview mean luma".into(), self.preview_mean.map(|m| format!("{m:.2}")).unwrap_or_else(|| "unavailable".into())),
            ("last frame sha256".into(), self.last_sha.clone().unwrap_or_else(|| "none".into())),
        ];
        let thermal = self.thermal.clone();
        snap.thermal = thermal.as_ref().map(|t| t.severity.clone()).unwrap_or_else(|| "unknown".into());
        snap.diagnostics = vec![
            ("app".into(), format!("deepsky-app {APP_VERSION}")),
            ("thermal".into(), thermal.as_ref().map(|t| format!("{} / {:?} C", t.severity, t.temperature_c)).unwrap_or_else(|| "unknown".into())),
            ("runner".into(), format!("{run_state:?}")),
            ("retries used".into(), self.run.as_ref().map(|r| r.runner.retries_used().to_string()).unwrap_or_else(|| "0".into())),
            ("preview failures".into(), self.preview_failures.to_string()),
        ];
        if self.sessions_checked.is_none_or(|at| at.elapsed() >= Duration::from_secs(3)) {
            self.sessions_cache = self.sessions();
            self.sessions_checked = Some(Instant::now());
        }
        snap.sessions = self.sessions_cache.clone();
        snap.rx_mbps = self.rx_mbps;
        snap.tx_mbps = self.tx_mbps;
        snap.dropped_raw = 0; // lossless path: failures are errors, never silent drops
        snap.dropped_preview = self.preview_failures;
        snap.storage_free = "unknown (not queryable portably)".into();
        snap.destination = self.destination.display().to_string();
        snap.auto_save = self.auto_save;
        snap.message = std::mem::take(&mut self.message);
        snap
    }
}

fn fatal(run: &mut ActiveRun, message: String) -> String {
    let _ = run.runner.stop();
    message
}

enum StepOutcome {
    Continue,
    /// Frame still exposing (or polled without landing): keep the run, no timer reset.
    Wait,
    Finished(String),
    Failed(String),
}

/// Run the worker until the UI disconnects. Handles every action, steps the
/// active sequence between actions, and always pushes a snapshot per event.
pub fn run_worker(rx: Receiver<UiAction>, tx: Sender<UiSnapshot>) {
    let mut worker = Worker::new();
    let mut next_preview = Instant::now();
    let mut deferred = Vec::new();
    let mut last_progress = Instant::now();
    let snapshot = worker.snapshot();
    if tx.send(snapshot).is_err() {
        return;
    }
    loop {
        if worker.poll_preview() {
            next_preview = Instant::now() + if worker.live_backoff == 0 {
                Duration::ZERO
            } else {
                Duration::from_millis(250 * u64::from(worker.live_backoff.min(20)))
            };
            if tx.send(worker.snapshot()).is_err() { break; }
        }
        let received = if worker.preview_in_flight.is_none() && !deferred.is_empty() {
            Ok(deferred.remove(0))
        } else {
            rx.recv_timeout(Duration::from_millis(16))
        };
        // Operations requiring the single camera link wait for its current RPC.
        // Pure control edits continue to be validated while preview is in flight.
        let received = match received {
            Ok(action) if worker.preview_in_flight.is_some()
                && (!deferred.is_empty() || !local_control(&action)) => {
                push_action(&mut deferred, action);
                continue;
            }
            other => other,
        };
        match received {
            Ok(UiAction::Shutdown) => {
                if let Some(run) = worker.run.take() {
                    let _ = worker.abort_run(run, "shutdown by user");
                }
                if let Some(mut backend) = worker.backend.take() {
                    let _ = backend.close();
                }
                let _ = tx.send(worker.snapshot());
                break;
            }
            Ok(action) => {
                // Drain a bounded batch before camera I/O. A pointer drag must
                // not enqueue one blocking preview for every intermediate value.
                let mut pending = Vec::new();
                push_action(&mut pending, action);
                for action in deferred.drain(..) {
                    push_action(&mut pending, action);
                }
                for action in rx.try_iter().take(255) {
                    push_action(&mut pending, action);
                }
                let mut shutdown = false;
                for action in pending {
                    if worker.preview_in_flight.is_some()
                        && (!deferred.is_empty() || !local_control(&action)) {
                        push_action(&mut deferred, action);
                        continue;
                    }
                    if action == UiAction::Shutdown {
                        worker.disconnect();
                        shutdown = true;
                        break;
                    }
                    worker.handle(action);
                }
                let snapshot = worker.snapshot();
                if tx.send(snapshot).is_err() {
                    break;
                }
                if shutdown { break; }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if worker.run.is_some() {
            worker.step();
            let snapshot = worker.snapshot();
            if tx.send(snapshot).is_err() {
                break;
            }
        } else if worker.preview_in_flight.is_some() && last_progress.elapsed() >= Duration::from_millis(50) {
            if tx.send(worker.snapshot()).is_err() { break; }
            last_progress = Instant::now();
        } else if worker.backend.is_some()
            && worker.open_camera_id.is_some()
            && Instant::now() >= next_preview
        {
            // Continuous live view: grab the next preview as soon as the
            // previous one lands. Nothing is ever written to disk here;
            // files are produced only by capture/sequence paths.
            worker.apply_live_controls();
            worker.start_preview();
        }
    }
}

fn local_control(action: &UiAction) -> bool {
    matches!(action, UiAction::SetExposure(_) | UiAction::SetIso(_)
        | UiAction::SetFocus(_) | UiAction::SetZoom(_) | UiAction::SetWhiteBalance(_)
        | UiAction::SetWbPreset(_) | UiAction::SetRaw(_) | UiAction::SetResolution(_, _)
        | UiAction::SetLocked(_) | UiAction::SetFrameCount(_) | UiAction::SetFrameType(_)
        | UiAction::SetAutoSave(_))
}

/// Replace only adjacent updates to the same control. Never move a setting
/// across capture, disconnect, source selection or another ordered operation.
fn push_action(pending: &mut Vec<UiAction>, action: UiAction) {
    let continuous = matches!(action, UiAction::SetExposure(_) | UiAction::SetIso(_)
        | UiAction::SetFocus(_) | UiAction::SetZoom(_) | UiAction::SetWhiteBalance(_)
        | UiAction::SetFrameCount(_));
    if continuous && pending.last().is_some_and(|previous|
        std::mem::discriminant(previous) == std::mem::discriminant(&action)) {
        *pending.last_mut().unwrap() = action;
    } else {
        pending.push(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_drag_without_crossing_capture_barrier() {
        let mut pending = Vec::new();
        for action in [UiAction::SetIso(100), UiAction::SetIso(200),
            UiAction::CaptureOne, UiAction::SetIso(400), UiAction::SetIso(800)] {
            push_action(&mut pending, action);
        }
        assert_eq!(pending, vec![UiAction::SetIso(200), UiAction::CaptureOne, UiAction::SetIso(800)]);
    }

    #[test]
    fn connect_starts_preview_and_disconnect_clears_it() {
        let mut worker = Worker::new();
        worker.connect_with(Source::Simulator { realtime: false });
        assert!(worker.open_camera_id.is_some(), "{}", worker.message);
        assert!(worker.preview.is_some(), "{}", worker.message);
        worker.disconnect();
        assert!(worker.preview.is_none());
        assert!(worker.histogram.is_empty());
    }

    #[test]
    fn preview_in_flight_keeps_connection_and_accepts_controls() {
        let mut worker = Worker::new();
        worker.connect_with(Source::Simulator { realtime: false });
        let revision = worker.preview_revision;
        worker.start_preview();
        assert!(worker.backend.is_none());
        assert!(worker.snapshot().connected);
        worker.handle(UiAction::SetIso(400));
        assert_eq!(worker.snapshot().iso, 400);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !worker.poll_preview() {
            assert!(Instant::now() < deadline, "preview stalled");
            std::thread::yield_now();
        }
        assert!(worker.backend.is_some());
        assert!(worker.preview_revision > revision);
        worker.disconnect();
    }

    #[test]
    fn source_env_parsing() {
        std::env::remove_var("DEEPSKY_SOURCE");
        assert!(matches!(source_from_env(), Source::Adb(None)));
        std::env::set_var("DEEPSKY_SOURCE", "tcp:127.0.0.1:9999");
        assert!(matches!(source_from_env(), Source::Tcp(_)));
        std::env::set_var("DEEPSKY_SOURCE", "bogus");
        assert!(matches!(source_from_env(), Source::Adb(None)));
        std::env::remove_var("DEEPSKY_SOURCE");
    }

    #[test]
    fn setting_validation_needs_announcement() {
        let mut worker = Worker::new();
        worker.set_exposure(1_000_000);
        assert_eq!(worker.message, "exposure range not announced");
        worker.set_zoom(0.5);
        assert!(worker.message.contains("outside announced range"));
    }

    #[test]
    fn preview_pipeline_delivers_image_and_histogram() {
        let dir = std::env::temp_dir().join(format!("deepsky-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut worker = Worker::new();
        worker.connect_with(Source::Simulator { realtime: false });
        assert!(worker.backend.is_some(), "simulator must connect");
        let id = worker.camera_id.clone();
        assert!(!id.is_empty(), "discovery must pick a camera");
        worker.select_camera(id);
        worker.destination = dir.clone();
        worker.capture_one();
        assert!(worker.preview.is_some(), "preview image missing: {}", worker.message);
        assert_eq!(worker.histogram.len(), 768, "RGB histogram expected");
        assert!(worker.preview_mean.is_some(), "preview mean missing");
        assert_eq!(worker.preview_failures, 0, "message: {}", worker.message);
        let png = worker.preview.as_ref().unwrap();
        assert!(png.bytes.starts_with(b"\x89PNG"), "PNG magic expected");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn select_tracks_open_state_and_never_double_opens() {
        let dir = std::env::temp_dir().join(format!("deepsky-reopen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut worker = Worker::new();
        worker.connect_with(Source::Simulator { realtime: false });
        let id = worker.camera_id.clone();
        worker.select_camera(id.clone());
        assert_eq!(worker.open_camera_id.as_deref(), Some(id.as_str()));
        assert!(worker.preview.is_some(), "live preview must start on select: {}", worker.message);
        // Selecting again is a no-op, not a double open.
        worker.select_camera(id.clone());
        assert!(worker.message.contains("already open"), "got: {}", worker.message);
        // A capture closes first, then prepares fresh: no InvalidState.
        worker.destination = dir.clone();
        worker.capture_one();
        assert!(worker.preview.is_some(), "capture must succeed after select: {}", worker.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_progress_flows_during_exposure() {
        let dir = std::env::temp_dir().join(format!("deepsky-progress-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut worker = Worker::new();
        worker.connect_with(Source::Simulator { realtime: true });
        assert!(worker.backend.is_some(), "simulator must connect");
        worker.destination = dir.clone();
        worker.frames_total = 1;
        worker.set_exposure(2_000_000_000);
        assert!(worker.message.is_empty(), "2 s must be announced: {}", worker.message);
        worker.focus_locked = false;
        worker.start_sequence();
        assert!(worker.focus_locked, "acquisition must freeze the selected manual focus");
        assert!(worker.run.is_some(), "run must start: {}", worker.message);
        // First step spawns the exposing thread; second step polls it.
        worker.step();
        assert!(worker.run.as_ref().is_some_and(|run| run.capture.is_some()));
        std::thread::sleep(Duration::from_millis(400));
        worker.step();
        match worker.frame_progress {
            Some(p) => assert!(p > 0.0 && p < 1.0, "progress must advance mid-exposure, got {p}"),
            None => panic!("frame_progress must be Some while exposing"),
        }
        let snapshot = worker.snapshot();
        assert_eq!(snapshot.frame_exposure_s, Some(2.0));
        assert!(snapshot.frame_elapsed_s.unwrap() >= 0.4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
