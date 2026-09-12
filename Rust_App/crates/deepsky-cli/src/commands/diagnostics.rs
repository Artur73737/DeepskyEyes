use deepsky_app::{controller::{connect_source, select_camera, build_request, AcquisitionOptions}, source::Source};
use deepsky_camera::model::{CameraSelection, ValidationPolicy};

pub fn run(source: &Source, method: &str, args: &[String]) -> Result<(),String> {
    let mut backend = connect_source(source).map_err(|e|e.to_string())?;
    let result = match method {
        "thermal" => serde_json::to_value(backend.thermal().map_err(|e|e.to_string())?).map_err(|e|e.to_string())?,
        "autofocus" => {
            let caps = backend.discover().map_err(|e|e.to_string())?;
            let camera = select_camera(&caps,crate::flag(args,"camera").as_deref()).map_err(|e|e.to_string())?;
            backend.open(&CameraSelection {camera_id:camera.camera_id.clone(),physical_id:None}).map_err(|e|e.to_string())?;
            let options = AcquisitionOptions {
                exposure_ns: camera.exposure_ns.ok_or("exposure range unavailable")?.min.max(10_000_000).min(camera.exposure_ns.unwrap().max),
                sensitivity: camera.sensitivity.ok_or("sensitivity range unavailable")?.min.try_into().map_err(|_|"sensitivity overflow")?,
                ..Default::default()
            };
            let request = build_request(camera,&options.spec()).map_err(|e|e.to_string())?;
            backend.configure(&request,ValidationPolicy::Reject).map_err(|e|e.to_string())?;
            let result = backend.autofocus_center();
            let closed = backend.close();
            let distance = result.map_err(|e|e.to_string())?;
            closed.map_err(|e|e.to_string())?;
            serde_json::json!({"camera_id":camera.camera_id,"focus_millidiopters":distance,"reuse":"pass --focus-mdiopt to capture/sequence; this connection is now closed"})
        },
        other => backend.diagnostic(other).map_err(|e|e.to_string())?
    };
    let text = serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?;
    if let Some(path) = crate::flag(args,"out") { std::fs::write(path,&text).map_err(|e|e.to_string())?; }
    println!("{text}");
    Ok(())
}
