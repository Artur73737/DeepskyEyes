//! deepsky-app entry point: desktop UI on the main thread, or one real
//! headless acquisition with --headless. Never prints a fake status.
use deepsky_app::{controller, source::Source};
#[cfg(feature = "desktop")]
use deepsky_app::worker;

fn main() {
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
        std::process::exit(2);
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
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
        out_dir: flag(args, "out").unwrap_or_else(|| "sessions".into()).into(),
        frames: parse_u64(args, "frames", 1)? as u32,
        exposure_ns: parse_u64(args, "exposure-ns", 15_000_000_000)?,
        sensitivity: parse_u64(args, "sensitivity", 800)? as u32,
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
