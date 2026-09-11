//! CLI capture/sequence: real acquisitions through the shared controller.
use std::path::PathBuf;

use deepsky_app::{
    controller::{run_acquisition, AcquisitionOptions, AcquisitionReport},
    source::Source,
};
use deepsky_session::calibration::SessionKind;

fn kind(args: &[String]) -> Result<SessionKind, String> {
    match crate::flag(args, "kind").as_deref().unwrap_or("light") {
        "light" => Ok(SessionKind::Light),
        "dark" => Ok(SessionKind::Dark),
        "flat" => Ok(SessionKind::Flat),
        "bias" => Ok(SessionKind::Bias),
        "test" => Ok(SessionKind::Test),
        other => Err(format!("unknown --kind '{other}': light|dark|flat|bias|test")),
    }
}

fn options(args: &[String], frames: u32) -> Result<AcquisitionOptions, String> {
    Ok(AcquisitionOptions {
        project: crate::flag(args, "project").ok_or("--project NAME required")?,
        out_dir: PathBuf::from(crate::flag(args, "out").unwrap_or_else(|| "sessions".into())),
        calibration: kind(args)?,
        frames,
        exposure_ns: crate::flag_or(args, "exposure-ns", 15_000_000_000)?,
        sensitivity: crate::flag_or(args, "sensitivity", 800)?,
        focus_millidiopters: crate::flag_or(args, "focus-mdiopt", 0)?,
        wb_kelvin: crate::flag_or(args, "wb-kelvin", 5_000)?,
        wb_preset: crate::flag(args, "wb-preset"),
        delay_ns: crate::flag_or(args, "delay-ns", 1_000_000_000)?,
        focus_locked: true,
        zoom_x1000: None,
        stream_override: None,
        raw: true,
    })
}

fn print_report(report: &AcquisitionReport) {
    println!("session: {}", report.session_dir.display());
    println!("session_id: {}", report.session_id);
    println!("identity: {} ({:?})", report.identity, report.origin);
    println!("frames_committed: {}", report.frames_committed);
    for (i, sha) in report.frame_sha256.iter().enumerate() {
        println!("frame[{i:04}]: sha256={sha}");
    }
    match report.preview_mean_luma {
        Some(mean) => println!("preview_mean_luma: {mean:.2}"),
        None => println!("preview_mean_luma: unavailable"),
    }
    match &report.preview_png {
        Some(path) => println!("preview_png: {}", path.display()),
        None => println!("preview_png: unavailable"),
    }
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
}

pub fn run_capture(source: &Source, args: &[String]) -> Result<(), String> {
    let report = run_acquisition(source, &options(args, 1)?).map_err(|e| e.to_string())?;
    print_report(&report);
    Ok(())
}

pub fn run_sequence(source: &Source, args: &[String]) -> Result<(), String> {
    let frames = crate::flag_or(args, "frames", 300)?;
    let report = run_acquisition(source, &options(args, frames)?).map_err(|e| e.to_string())?;
    print_report(&report);
    Ok(())
}
