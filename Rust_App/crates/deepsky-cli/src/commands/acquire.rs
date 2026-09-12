//! CLI capture/sequence: real acquisitions through the shared controller.
use std::path::PathBuf;

use deepsky_app::{
    controller::{
        connect_source, parse_zoom, resolve_stream, run_acquisition, select_camera,
        AcquisitionOptions, AcquisitionReport,
    },
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

/// Selection flags that need announced capabilities to resolve (`--camera`,
/// `--zoom`, `--stream`). Discovery here is a cheap read-only round trip; the
/// acquisition itself reconnects and re-validates everything with the Reject
/// policy, so nothing resolved here is trusted downstream.
fn selection(
    source: &Source,
    args: &[String],
) -> Result<
    (
        Option<String>,
        Option<u64>,
        Option<deepsky_camera::model::StreamConfiguration>,
    ),
    String,
> {
    let camera_id = crate::flag(args, "camera");
    let zoom = parse_zoom(
        crate::flag(args, "zoom").as_deref(),
        crate::flag(args, "zoom-x1000").as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let stream_spec = crate::flag(args, "stream");
    if camera_id.is_none() && zoom.is_none() && stream_spec.is_none() {
        return Ok((None, None, None));
    }
    let mut backend = connect_source(source).map_err(|e| e.to_string())?;
    let caps_list = backend.discover().map_err(|e| e.to_string())?;
    let caps = select_camera(&caps_list, camera_id.as_deref())
        .map_err(|e| e.to_string())?
        .clone();
    let stream = stream_spec
        .map(|spec| resolve_stream(&caps, &spec).map_err(|e| e.to_string()))
        .transpose()?;
    Ok((camera_id, zoom, stream))
}

fn options(
    source: &Source,
    args: &[String],
    frames: u32,
) -> Result<AcquisitionOptions, String> {
    let (camera_id, zoom_x1000, stream_override) = selection(source, args)?;
    Ok(AcquisitionOptions {
        strict_results: args.iter().any(|v|v == "--strict-results"),
        controls: super::controls::parse(args)?,
        project: crate::flag(args, "project").ok_or("--project NAME required")?,
        out_dir: crate::flag(args, "out").map(PathBuf::from).unwrap_or_else(deepsky_app::controller::default_capture_dir),
        camera_id,
        calibration: kind(args)?,
        frames,
        exposure_ns: crate::flag_or(args, "exposure-ns", 15_000_000_000)?,
        sensitivity: crate::flag_or(args, "sensitivity", 800)?,
        focus_millidiopters: crate::flag_or(args, "focus-mdiopt", 0)?,
        wb_kelvin: crate::flag_or(args, "wb-kelvin", 5_000)?,
        wb_preset: crate::flag(args, "wb-preset"),
        delay_ns: crate::flag_or(args, "delay-ns", 0)?,
        focus_locked: true,
        zoom_x1000,
        stream_override,
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
    let report = run_acquisition(source, &options(source, args, 1)?).map_err(|e| e.to_string())?;
    print_report(&report);
    Ok(())
}

pub fn run_sequence(source: &Source, args: &[String]) -> Result<(), String> {
    let frames = crate::flag_or(args, "frames", 300)?;
    let report =
        run_acquisition(source, &options(source, args, frames)?).map_err(|e| e.to_string())?;
    print_report(&report);
    Ok(())
}
