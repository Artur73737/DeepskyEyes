//! deepsky-app entry point: desktop UI on the main thread, or one real
//! headless acquisition with --headless. Never prints a fake status.
//!
//! GUI subsystem on Windows: double-clicking the exe opens no terminal.
//! Output still reaches the calling console for --headless runs.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use deepsky_app::{controller, source::Source};
#[cfg(feature = "desktop")]
use deepsky_app::worker;

fn main() {
    install_crash_log();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--headless") {
        std::process::exit(match headless(&args) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("deepsky-app: error: {message}");
                1
            }
        });
    }
    #[cfg(feature = "desktop")]
    {
        let (snapshot_tx, snapshot_rx) = std::sync::mpsc::channel();
        let (action_tx, action_rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("deepsky-worker".into())
            .spawn(move || worker::run_worker(action_rx, snapshot_tx))
            .expect("worker thread spawns");
        deepsky_ui::run(deepsky_ui::UiSnapshot::default(), snapshot_rx, action_tx);
        return;
    }
    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("deepsky-app {}: headless build (desktop feature off).", controller::APP_VERSION);
        eprintln!("  deepsky-app --headless --out DIR --project NAME [--frames N] [...]");
        eprintln!("  or drive it with deepsky-eyes, or rebuild with --features desktop.");
        pause_if_terminal();
        std::process::exit(2);
    }
}

/// Keep the console window open on double-click so the message above stays
/// readable. Scripts and pipes are unaffected (no terminal, no wait).
#[cfg(not(feature = "desktop"))]
fn pause_if_terminal() {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        eprintln!("Press Enter to close.");
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }
}

/// Panics are invisible without a console: append them to %TEMP% so a crash
/// always leaves evidence (send deepsky-crash.log with bug reports).
fn install_crash_log() {
    std::panic::set_hook(Box::new(|info| {
        let line = format!("{:?} panic: {info}\n", std::time::SystemTime::now());
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("deepsky-crash.log"))
        {
            use std::io::Write;
            let _ = file.write_all(line.as_bytes());
        }
    }));
}

fn flag(args: &[String], name: &str) -> Option<String> {    let mut i = 0;
    while i < args.len() {
        if args[i] == format!("--{name}") {
            return args.get(i + 1).cloned();
        }
        if let Some(value) = args[i].strip_prefix(&format!("--{name}=")) {
            return Some(value.to_string());
        }
        i += 1;
    }
    None
}

fn parse_u64(args: &[String], name: &str, default: u64) -> Result<u64, String> {
    match flag(args, name) {
        Some(value) => value.parse().map_err(|e| format!("--{name}: {e}")),
        None => Ok(default),
    }
}

/// One real acquisition with UI-free flags. Source defaults to the explicit
/// SYNTHETIC simulator; use --source tcp:.../adb for hardware paths.
fn headless(args: &[String]) -> Result<(), String> {
    let source = match flag(args, "source").as_deref().unwrap_or("sim") {
        "sim" => Source::Simulator { realtime: false },
        "sim:realtime" => Source::Simulator { realtime: true },
        s if s.starts_with("tcp:") => Source::Tcp(s["tcp:".len()..].to_string()),
        "adb" => Source::Adb(None),
        s if s.starts_with("adb:") => Source::Adb(Some(s["adb:".len()..].to_string())),
        other => return Err(format!("unknown --source '{other}'")),
    };
    let opts = controller::AcquisitionOptions {
        project: flag(args, "project").ok_or("--project NAME required")?,
        out_dir: flag(args, "out").map(Into::into).unwrap_or_else(controller::default_capture_dir),
        camera_id: flag(args, "camera"),
        frames: parse_u64(args, "frames", 1)? as u32,
        exposure_ns: parse_u64(args, "exposure-ns", 15_000_000_000)?,
        sensitivity: parse_u64(args, "sensitivity", 800)? as u32,
        zoom_x1000: controller::parse_zoom(flag(args, "zoom").as_deref(), flag(args, "zoom-x1000").as_deref()).map_err(|e| e.to_string())?,
        ..Default::default()
    };
    let report = controller::run_acquisition(&source, &opts).map_err(|e| e.to_string())?;
    println!("session: {}", report.session_dir.display());
    println!("frames_committed: {}", report.frames_committed);
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
    Ok(())
}
