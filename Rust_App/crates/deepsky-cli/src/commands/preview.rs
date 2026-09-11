//! CLI preview: open + configure + one preview frame, saved as PNG.
//! Exercises the exact path the desktop live preview uses.
use std::path::PathBuf;

use deepsky_app::{
    controller::{build_request, connect_source, encode_png_rgb8, CaptureSpec},
    source::Source,
};
use deepsky_camera::model::{CameraSelection, Origin, ValidationPolicy};
use deepsky_preview::decoder::preview_to_rgb8;

pub fn run(source: &Source, args: &[String]) -> Result<(), String> {
    let mut backend = connect_source(source).map_err(|e| e.to_string())?;
    let caps_list = backend.discover().map_err(|e| e.to_string())?;
    let caps = match crate::flag(args, "camera") {
        Some(id) => caps_list
            .iter()
            .find(|c| c.camera_id == id)
            .ok_or_else(|| format!("camera '{id}' not announced"))?,
        None => caps_list
            .iter()
            .find(|c| c.origin == Some(Origin::Device))
            .or_else(|| caps_list.first())
            .ok_or_else(|| "device announced zero cameras".to_string())?,
    };
    let selection = CameraSelection { camera_id: caps.camera_id.clone(), physical_id: None };
    backend.open(&selection).map_err(|e| format!("open: {e}"))?;
    let spec = CaptureSpec {
        exposure_ns: crate::flag_or(args, "exposure-ns", 1_000_000_000)?,
        sensitivity: crate::flag_or(args, "sensitivity", 800)?,
        focus_millidiopters: crate::flag_or(args, "focus-mdiopt", 0)?,
        focus_locked: true,
        wb_kelvin: crate::flag_or(args, "wb-kelvin", 5_000)?,
        wb_preset: crate::flag(args, "wb-preset"),
        zoom_x1000: None,
        stream_override: None,
        raw: true,
    };
    let request = build_request(caps, &spec).map_err(|e| e.to_string())?;
    backend
        .configure(&request, ValidationPolicy::Reject)
        .map_err(|e| format!("configure: {e}"))?;
    let frame = backend.preview().map_err(|e| format!("preview: {e}"))?;
    let rgb = preview_to_rgb8(&frame).map_err(|e| format!("decode: {e}"))?;
    let png = encode_png_rgb8(&rgb.rgb, rgb.width, rgb.height).map_err(|e| format!("png: {e}"))?;
    let out = match crate::flag(args, "out") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(format!("preview-{}x{}.png", rgb.width, rgb.height)),
    };
    std::fs::write(&out, &png).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!("preview: {} ({}x{}, mean luma {:.1})", out.display(), rgb.width, rgb.height, rgb.mean_luma());
    Ok(())
}
