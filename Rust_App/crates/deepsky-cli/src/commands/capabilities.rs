//! CLI capabilities: announced capability snapshot as JSON, verbatim.
use deepsky_app::{controller::connect_source, source::Source};

pub fn run(source: &Source, args: &[String]) -> Result<(), String> {
    let id = crate::flag(args, "camera").ok_or("capabilities needs --camera ID")?;
    let mut backend = connect_source(source).map_err(|e| e.to_string())?;
    let caps = backend.discover().map_err(|e| e.to_string())?;
    let camera = caps.iter().find(|c| c.camera_id == id).ok_or_else(|| format!("camera '{id}' not announced"))?;
    println!("{}", serde_json::to_string_pretty(camera).map_err(|e| e.to_string())?);
    Ok(())
}
