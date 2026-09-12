//! Explicit JSON request path for all portable controls, including JPEG.
//! Scientific sequences retain their stricter RAW/fixed-focus preflight.
use std::io::Write;
use deepsky_app::{controller::{connect_source, build_request, resolve_stream, select_camera, AcquisitionOptions},source::Source};
use deepsky_camera::model::{CaptureRequest, ValidationPolicy};

pub fn template(source: &Source, args: &[String]) -> Result<(),String> {
    let mut backend = connect_source(source).map_err(|e|e.to_string())?;
    let caps = backend.discover().map_err(|e|e.to_string())?;
    let camera = select_camera(&caps,crate::flag(args,"camera").as_deref()).map_err(|e|e.to_string())?;
    let options = AcquisitionOptions {
        exposure_ns: camera.exposure_ns.ok_or("exposure range unavailable")?.min.max(10_000_000).min(camera.exposure_ns.unwrap().max),
        sensitivity: camera.sensitivity.ok_or("sensitivity range unavailable")?.min.try_into().map_err(|_|"sensitivity overflow")?,
        stream_override: crate::flag(args,"stream").map(|v|resolve_stream(camera,&v).map_err(|e|e.to_string())).transpose()?,
        ..Default::default()
    };
    let request = build_request(camera,&options.spec()).map_err(|e|e.to_string())?;
    let text = serde_json::to_string_pretty(&request).map_err(|e|e.to_string())?;
    if let Some(path) = crate::flag(args,"out") { write_new(&path,text.as_bytes())?; }
    println!("{text}"); Ok(())
}

fn write_new(path: &str, bytes: &[u8]) -> Result<(),String> {
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|format!("{path}: {e}"))?;
    file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())
}

pub fn inspect(args: &[String]) -> Result<(),String> {
    let path = crate::flag(args,"path").ok_or("--path DIR required")?;
    let store = deepsky_session::store::SessionStore::open(path).map_err(|e|e.to_string())?;
    let scan = store.scan().map_err(|e|e.to_string())?;
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "manifest":store.manifest(),"verified":scan.verified,"missing":scan.missing,
        "corrupt":scan.corrupt,"untracked":scan.untracked,"frames_remaining":scan.frames_remaining,
        "integrity_ok":scan.can_resume(),"automatic_resume":false
    })).map_err(|e|e.to_string())?);
    if !scan.can_resume() { return Err("session integrity check failed; no files changed".into()); }
    Ok(())
}

pub fn run(source: &Source, args: &[String]) -> Result<(),String> {
    let path = crate::flag(args,"request").ok_or("--request FILE required")?;
    let output = crate::flag(args,"out").ok_or("--out FILE required (payload format follows request.stream)")?;
    let sidecar = format!("{output}.json");
    if std::path::Path::new(&output).exists() || std::path::Path::new(&sidecar).exists() { return Err("output already exists; refusing overwrite".into()); }
    let request: CaptureRequest = serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let mut backend = connect_source(source).map_err(|e|e.to_string())?;
    let caps = backend.discover().map_err(|e|e.to_string())?;
    let camera = select_camera(&caps,Some(&request.selection.camera_id)).map_err(|e|e.to_string())?;
    camera.validate_request(&request,ValidationPolicy::Reject).map_err(|e|e.to_string())?;
    backend.open(&request.selection).map_err(|e|e.to_string())?;
    let result = (|| {
        backend.configure(&request,ValidationPolicy::Reject).map_err(|e|e.to_string())?;
        let frame = backend.capture().map_err(|e|e.to_string())?;
        let metadata = serde_json::to_vec_pretty(&frame.metadata).map_err(|e|e.to_string())?;
        write_new(&output,&frame.payload)?;
        write_new(&sidecar,&metadata)?;
        println!("{}",serde_json::json!({"payload":output,"metadata":sidecar,"bytes":frame.payload.len(),"origin":frame.metadata.origin}));
        Ok(())
    })();
    let closed = backend.close().map_err(|e|e.to_string());
    result.and(closed)
}
