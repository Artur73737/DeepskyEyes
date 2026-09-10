//! CLI discover: announced cameras from a real backend, nothing invented.
use deepsky_app::{controller::connect_source, source::Source};
use deepsky_camera::discovery::summarize;

pub fn run(source: &Source) -> Result<(), String> {
    let mut backend = connect_source(source).map_err(|e| e.to_string())?;
    let caps = backend.discover().map_err(|e| e.to_string())?;
    if caps.is_empty() {
        return Err("device announced zero cameras".into());
    }
    for camera in summarize(&caps) {
        let origin = caps.iter().find(|c| c.camera_id == camera.camera_id).and_then(|c| c.origin.clone());
        println!(
            "{} origin={:?} lens={} facing={} raw={} manual={} streams={} physical=[{}]",
            camera.camera_id,
            origin,
            camera.lens.label(),
            camera.lens_facing,
            camera.raw_supported,
            camera.manual_sensor,
            camera.stream_count,
            camera.physical_ids.join(","),
        );
    }
    Ok(())
}
