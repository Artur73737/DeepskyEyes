use deepsky_camera::{backend::CameraBackend, *};
use deepsky_testkit::{SimulatorBackend as Sim, TimingMode, failure_injection::InjectedFailure};
fn ready() -> Sim { let mut s = Sim::default(); let r = Sim::default_request(); s.open(&r.selection).unwrap(); s.configure(&r, ValidationPolicy::Reject).unwrap(); s }
#[test]
fn deterministic_raw_preview_and_serialization() {
    let mut a = ready(); let mut b = ready();
    let preview = a.preview().unwrap(); assert_eq!(preview.payload.len(), 128*96);
    let frame = a.capture().unwrap(); assert_eq!(frame, b.capture().unwrap()); assert_eq!(frame.payload.len(), 128*96*2);
    assert!(frame.metadata.identity.contains("SYNTHETIC")); assert_eq!(frame.metadata.origin, Origin::Synthetic);
    let pixels: Vec<u16> = frame.payload.chunks_exact(2).map(|p| u16::from_le_bytes([p[0],p[1]])).collect();
    assert!(*pixels.iter().max().unwrap() > 20000); assert!(*pixels.iter().min().unwrap() >= 256);
    assert_eq!(frame.metadata.end_ns, 1_500_000); assert_ne!(frame.payload, a.capture().unwrap().payload);
    let json = serde_json::to_string(&frame).unwrap(); assert_eq!(frame, serde_json::from_str::<CapturedFrame>(&json).unwrap());
    let caps = Sim::capabilities(); assert_eq!(caps, serde_json::from_str(&serde_json::to_string(&caps).unwrap()).unwrap());
}
#[test]
fn rejection_clamp_and_transactional_configuration() {
    let mut sim = ready(); let mut r = Sim::default_request(); r.settings.sensitivity = Some(99999);
    assert_eq!(sim.configure(&r, ValidationPolicy::Reject).unwrap_err().code, ErrorCode::OutOfRange);
    assert_eq!(sim.capture().unwrap().metadata.reported.sensitivity, Some(100));
    let result = sim.configure(&r, ValidationPolicy::Clamp).unwrap(); assert_eq!(result.requested.settings.sensitivity, Some(99999)); assert_eq!(result.applied.settings.sensitivity, Some(6400)); assert_eq!(result.adjustments.len(), 1);
    assert_eq!(sim.capture().unwrap().metadata.reported.sensitivity, Some(6400));
    r.settings.stream.as_mut().unwrap().width = 50; assert!(sim.configure(&r, ValidationPolicy::Clamp).is_err());
}
#[test]
fn unknown_capabilities_never_support_controls() {
    let r = Sim::default_request(); let empty = CameraCapabilities { camera_id: r.selection.camera_id.clone(), ..Default::default() };
    assert_eq!(empty.validate_request(&r, ValidationPolicy::Clamp).unwrap_err().code, ErrorCode::Unsupported);
    let mut caps = Sim::capabilities(); caps.focus_millidiopters = None; assert!(caps.validate_request(&r, ValidationPolicy::Reject).is_err());
    caps = Sim::capabilities(); caps.exposure_ns = Some(ValueRange { min: 5, max: 1 }); assert_eq!(caps.validate_request(&r, ValidationPolicy::Clamp).unwrap_err().code, ErrorCode::InvalidCapabilities);
    assert!(!deepsky_camera::controls::zoom::ZoomControl::new_1x().is_supported(1000));
}
#[test]
fn focus_changes_stars_and_no_implicit_af() {
    let mut sim = ready(); let sharp = sim.capture().unwrap(); let mut r = Sim::default_request();
    r.settings.focus = Some(FocusRequest::Auto { mode: "auto".into(), locked: true }); assert!(sim.configure(&r, ValidationPolicy::Reject).is_err());
    r.settings.focus = Some(FocusRequest::Manual { millidiopters: 3000, locked: true }); sim.configure(&r, ValidationPolicy::Reject).unwrap(); let blurred = sim.capture().unwrap();
    let peak = |f: &CapturedFrame| f.payload.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).max().unwrap(); assert!(peak(&sharp) > peak(&blurred)*4);
}
#[test]
fn lifecycle_and_failures() {
    let mut s = Sim::default(); assert!(s.capture().is_err()); assert!(s.preview().is_err());
    let r = Sim::default_request(); s.open(&r.selection).unwrap(); assert!(s.open(&r.selection).is_err()); s.configure(&r, ValidationPolicy::Reject).unwrap();
    s.inject_failure(InjectedFailure::TransportTimeout); assert_eq!(s.capture().unwrap_err().code, ErrorCode::Timeout); assert_eq!(s.elapsed_ns(), 0); s.capture().unwrap();
    s.inject_failure(InjectedFailure::ThermalWarning); assert_eq!(s.thermal().unwrap().severity, "warning");
    s.inject_failure(InjectedFailure::UsbDisconnect); assert_eq!(s.capture().unwrap_err().code, ErrorCode::Disconnected); assert!(s.capture().is_err());
    s.open(&r.selection).unwrap(); assert!(s.capture().is_err()); s.close().unwrap(); s.close().unwrap();
}
#[test]
fn realtime_waits_but_preserves_simulated_timestamps() {
    let mut s = Sim::new(TimingMode::Realtime); let r = Sim::default_request(); s.open(&r.selection).unwrap(); s.configure(&r, ValidationPolicy::Reject).unwrap();
    let start = std::time::Instant::now(); let frame = s.capture().unwrap(); assert!(start.elapsed() >= std::time::Duration::from_nanos(1_500_000)); assert_eq!(frame.metadata.end_ns, 1_500_000);
}
#[test]
fn wb_processing_timing_crop_and_physical_validation() {
    let mut c = Sim::capabilities(); let mut r = Sim::default_request(); r.settings.processing.insert("noise_reduction".into(), "high_quality".into()); assert!(c.validate_request(&r, ValidationPolicy::Clamp).is_err()); r.settings.processing.clear();
    r.settings.white_balance = Some(WhiteBalanceRequest::Manual { gains_x1000: [0;4], transform_millionths: [0;9] }); let clamped = c.validate_request(&r, ValidationPolicy::Clamp).unwrap(); assert_eq!(clamped.adjustments.len(), 4);
    r.settings.white_balance = None; r.settings.exposure_ns = Some(2_000_000); assert!(c.validate_request(&r, ValidationPolicy::Clamp).is_err()); r.settings.exposure_ns = Some(1_000_000);
    r.settings.crop = Some(CropRect { x: u32::MAX, y: 0, width: 2, height: 1 }); c.crop_supported = Some(true); c.active_array = Some(CropRect { x: 0, y: 0, width: 128, height: 96 }); assert!(c.validate_request(&r, ValidationPolicy::Reject).is_err()); r.settings.crop = None;
    let mut logical = CameraCapabilities { camera_id: "logical".into(), logical: Some(true), ..Default::default() }; logical.physical_cameras.push(PhysicalCamera { id: c.camera_id.clone(), directly_openable: Some(false), routable: true });
    r.selection = CameraSelection { camera_id: "logical".into(), physical_id: Some(c.camera_id.clone()) }; assert!(logical.validate_request(&r, ValidationPolicy::Reject).is_err()); let outcome = logical.validate_physical_request(&c, &r, ValidationPolicy::Reject).unwrap(); assert_eq!(outcome.applied.selection, r.selection);
    c.manual_sensor = None; assert!(logical.validate_physical_request(&c, &r, ValidationPolicy::Reject).is_err());
}
